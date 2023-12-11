use std::{cell::RefCell, future::Future, pin::Pin, rc::Rc, sync::Mutex};

use bevy::{
    asset::{AssetHandleProvider, AssetLoader, AsyncReadExt},
    prelude::*,
    tasks::AsyncComputeTaskPool,
    utils::HashMap,
};

pub mod prelude {
    pub use crate::{Audio, AudioInstance, AudioPlugin, AudioSource};
}

pub struct AudioPlugin;

impl bevy::app::Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, setup);
        app.add_systems(Update, start_queued);
        app.add_systems(Update, start_task_pool);
        app.init_asset::<AudioSource>();
        app.init_asset::<AudioInstance>();
        app.register_asset_loader(SoundLoader);
    }
}

struct SoundLoader;

struct AudioSystem {
    audio: geng_audio::Audio,
    sounds: RefCell<Vec<geng_audio::Sound>>,
    effects: RefCell<HashMap<AssetId<AudioInstance>, geng_audio::SoundEffect>>,
}

struct TaskPool {
    tasks: Mutex<
        Vec<
            Box<
                dyn Send
                    + for<'a> FnOnce(&'a AudioSystem) -> Pin<Box<dyn Future<Output = ()> + 'a>>,
            >,
        >,
    >,
}

static TASK_POOL: once_cell::sync::Lazy<TaskPool> =
    once_cell::sync::Lazy::new(|| TaskPool { tasks: default() });

fn start_task_pool(audio_system: NonSend<Rc<AudioSystem>>) {
    let task_pool = AsyncComputeTaskPool::get();
    for task in TASK_POOL.tasks.lock().unwrap().drain(..) {
        let audio_system = audio_system.clone();
        task_pool
            .spawn_local(async move { task(&audio_system).await })
            .detach();
    }
}

async fn with_audio_system<T: Send + Sync + 'static>(
    f: impl 'static + Send + for<'a> FnOnce(&'a AudioSystem) -> Pin<Box<dyn Future<Output = T> + 'a>>,
) -> Result<T, async_oneshot::Closed> {
    let (mut sender, receiver) = async_oneshot::oneshot();
    TASK_POOL
        .tasks
        .lock()
        .unwrap()
        .push(Box::new(move |audio: &AudioSystem| {
            Box::pin(async move {
                let thing = f(audio).await;
                let _ = sender.send(thing);
            })
        }));
    receiver.await
}

impl AssetLoader for SoundLoader {
    type Asset = AudioSource;
    type Settings = ();
    type Error = anyhow::Error;
    fn load<'a>(
        &'a self,
        reader: &'a mut bevy::asset::io::Reader,
        _settings: &'a Self::Settings,
        _load_context: &'a mut bevy::asset::LoadContext,
    ) -> bevy::utils::BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut buf = Vec::new();
            reader.read_to_end(&mut buf).await?;
            with_audio_system(|audio_system| {
                Box::pin(async move {
                    let source = audio_system.audio.decode_bytes(buf).await?;
                    let mut sounds = audio_system.sounds.borrow_mut();
                    let sound_index = sounds.len();
                    sounds.push(source);
                    Ok(AudioSource { sound_index })
                })
            })
            .await
            .map_err(|_: async_oneshot::Closed| anyhow::anyhow!("channel closed"))?
        })
    }

    fn extensions(&self) -> &[&str] {
        &["wav", "mp3", "ogg"]
    }
}

fn setup(world: &mut World) {
    world.insert_non_send_resource(Rc::new(AudioSystem {
        audio: geng_audio::Audio::new(),
        effects: default(),
        sounds: default(),
    }));

    let instances = world.get_resource::<Assets<AudioInstance>>().unwrap();

    world.insert_resource(Audio {
        instance_handle_provider: instances.get_handle_provider(),
        new_instances: default(),
    });
}

#[derive(Resource)]
pub struct Audio {
    instance_handle_provider: AssetHandleProvider,
    new_instances: Mutex<Vec<QueuedAudio>>,
}

fn start_queued(
    mut audio: ResMut<Audio>,
    sources: ResMut<Assets<AudioSource>>,
    mut instances: ResMut<Assets<AudioInstance>>,
    mut events: EventReader<AssetEvent<AudioInstance>>,
) {
    let audio = &mut *audio;

    let task_pool = AsyncComputeTaskPool::get();

    for queued in std::mem::take(audio.new_instances.get_mut().unwrap()) {
        let Some(source) = sources.get(queued.source.clone()) else {
            audio.new_instances.get_mut().unwrap().push(queued);
            continue;
        };
        let sound_index = source.sound_index;
        let id = queued.handle.id();
        task_pool
            .spawn(with_audio_system(move |audio_system| {
                Box::pin(async move {
                    let mut sounds = audio_system.sounds.borrow_mut();
                    let sound = &mut sounds[sound_index];
                    sound.set_looped(queued.looped); // LUL
                    let mut effect = sound.effect();
                    effect.set_volume(queued.volume);
                    effect.play();
                    let mut effects = audio_system.effects.borrow_mut();
                    effects.insert(id, effect);
                })
            }))
            .detach();
        instances.insert(queued.handle, AudioInstance { need_stop: false });
    }

    for event in events.read() {
        match event {
            AssetEvent::Added { .. } => {}
            AssetEvent::Modified { id } => {
                let id = *id;
                if let Some(instance) = instances.get(id) {
                    if instance.need_stop {
                        task_pool
                            .spawn(with_audio_system(move |audio_system| {
                                Box::pin(async move {
                                    let mut effects = audio_system.effects.borrow_mut();
                                    if let Some(effect) = effects.get_mut(&id) {
                                        effect.stop();
                                    }
                                })
                            }))
                            .detach();
                    }
                }
            }
            AssetEvent::Removed { id } => {
                let id = *id;
                task_pool
                    .spawn(with_audio_system(move |audio_system| {
                        Box::pin(async move {
                            let mut effects = audio_system.effects.borrow_mut();
                            effects.remove(&id);
                        })
                    }))
                    .detach();
            }
            AssetEvent::LoadedWithDependencies { .. } => {}
        }
    }
}

struct QueuedAudio {
    volume: f64,
    looped: bool,
    source: Handle<AudioSource>,
    handle: Handle<AudioInstance>,
}

pub struct PlayAudioCommand<'a> {
    audio: &'a Audio,
    queued: Option<QueuedAudio>,
}

impl PlayAudioCommand<'_> {
    pub fn with_volume(&mut self, volume: f64) -> &mut Self {
        self.queued.as_mut().unwrap().volume = volume;
        self
    }
    pub fn looped(&mut self) -> &mut Self {
        self.queued.as_mut().unwrap().looped = true;
        self
    }
    pub fn handle(&self) -> Handle<AudioInstance> {
        self.queued.as_ref().unwrap().handle.clone()
    }
}

impl Drop for PlayAudioCommand<'_> {
    fn drop(&mut self) {
        self.audio
            .new_instances
            .lock()
            .unwrap()
            .push(self.queued.take().unwrap());
    }
}

impl Audio {
    pub fn play(&self, source: Handle<AudioSource>) -> PlayAudioCommand<'_> {
        let handle = self.instance_handle_provider.reserve_handle();
        PlayAudioCommand {
            audio: self,
            queued: Some(QueuedAudio {
                source,
                looped: false,
                volume: 1.0,
                handle: handle.typed(),
            }),
        }
    }
}

#[derive(Asset, Reflect)]
pub struct AudioInstance {
    need_stop: bool,
}

impl AudioInstance {
    pub fn stop(&mut self) {
        self.need_stop = true;
    }
}

#[derive(Asset, TypePath)]
pub struct AudioSource {
    sound_index: usize,
}
