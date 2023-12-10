use bevy::{
    audio::{Volume, VolumeLevel},
    prelude::*,
};

use crate::{
    buttons::Disabled,
    game::{BlockingGhost, EntState, EntType, Placeholder},
};

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_audio);
        app.add_systems(Update, audio_buttons);
        app.add_systems(Update, audio_construct);
        app.add_systems(Update, audio_constructed);
    }
}

#[derive(Resource)]
struct AudioSources {
    music: Handle<AudioSource>,
    crab_rave: Handle<AudioSource>,
    button_hover: Handle<AudioSource>,
    button_press: Handle<AudioSource>,
    construct: Handle<AudioSource>,
    construct_road: Handle<AudioSource>,
    constructed: Handle<AudioSource>,
}

fn setup_audio(mut commands: Commands, asset_server: Res<AssetServer>) {
    let audio_sources = AudioSources {
        music: asset_server.load("crabBOP.ogg"),
        crab_rave: asset_server.load("crabJAM.ogg"),
        button_hover: asset_server.load("button_hover.ogg"),
        button_press: asset_server.load("button_press.ogg"),
        construct: asset_server.load("construct.ogg"),
        construct_road: asset_server.load("road.ogg"),
        constructed: asset_server.load("constructed.ogg"),
    };

    commands.spawn(AudioBundle {
        source: audio_sources.music.clone(),
        settings: PlaybackSettings {
            mode: bevy::audio::PlaybackMode::Loop,
            volume: Volume::Relative(VolumeLevel::new(0.35)),
            ..default()
        },
    });

    commands.insert_resource(audio_sources);
}

fn audio_buttons(
    mut commands: Commands,
    mut audio_sources: Res<AudioSources>,
    mut button_interactions: Query<
        (&Interaction, Option<&Disabled>),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, disabled) in &mut button_interactions {
        if !matches!(disabled, Some(Disabled(true))) {
            match *interaction {
                Interaction::Pressed => {
                    commands.spawn(AudioBundle {
                        source: audio_sources.button_press.clone(),
                        ..default()
                    });
                }
                Interaction::Hovered => {
                    commands.spawn(AudioBundle {
                        source: audio_sources.button_hover.clone(),
                        ..default()
                    });
                }
                Interaction::None => {}
            }
        }
    }
}

fn audio_construct(
    mut commands: Commands,
    mut audio_sources: Res<AudioSources>,
    new_placeholders: Query<&Placeholder, Added<Placeholder>>,
) {
    for placeholder in new_placeholders.iter() {
        match placeholder.0 {
            EntType::Harvester
            | EntType::Base
            | EntType::Storage
            | EntType::House
            | EntType::UpgradeInventory
            | EntType::BuilderAcademy
            | EntType::Monument => {
                commands.spawn(AudioBundle {
                    source: audio_sources.construct.clone(),
                    settings: PlaybackSettings {
                        volume: Volume::Relative(VolumeLevel::new(2.0)),
                        ..default()
                    },
                    ..default()
                });
            }
            EntType::Road => {
                commands.spawn(AudioBundle {
                    source: audio_sources.construct_road.clone(),
                    settings: PlaybackSettings {
                        volume: Volume::Relative(VolumeLevel::new(2.0)),
                        ..default()
                    },
                    ..default()
                });
            }
            _ => {}
        }
    }
}

fn audio_constructed(
    mut commands: Commands,
    mut audio_sources: Res<AudioSources>,
    new_entities: Query<&EntType, Added<EntType>>,
) {
    for entity in new_entities.iter() {
        if matches!(
            entity,
            EntType::Harvester
                | EntType::Base
                | EntType::Storage
                | EntType::House
                | EntType::UpgradeInventory
                | EntType::BuilderAcademy
                | EntType::Monument
        ) {
            commands.spawn(AudioBundle {
                source: audio_sources.constructed.clone(),
                settings: PlaybackSettings {
                    volume: Volume::Relative(VolumeLevel::new(0.25)),
                    ..default()
                },
                ..default()
            });

            // dont play audio for more than once entity
            // for this step ( overlapping sounds are loud )
            break;
        }
    }
}
