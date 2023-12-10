use bevy::prelude::*;
use bevy_kira_audio::prelude::*;

use crate::{
    buttons::Disabled,
    game::{EntType, Placeholder},
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

fn setup_audio(mut commands: Commands, asset_server: Res<AssetServer>, audio: Res<Audio>) {
    let audio_sources = AudioSources {
        music: asset_server.load("crabBOP.ogg"),
        crab_rave: asset_server.load("crabJAM.ogg"),
        button_hover: asset_server.load("button_hover.ogg"),
        button_press: asset_server.load("button_press.ogg"),
        construct: asset_server.load("construct.ogg"),
        construct_road: asset_server.load("road.ogg"),
        constructed: asset_server.load("constructed.ogg"),
    };

    audio
        .play(audio_sources.music.clone())
        .looped()
        .with_volume(0.35);

    commands.insert_resource(audio_sources);
}

fn audio_buttons(
    mut commands: Commands,
    audio_sources: Res<AudioSources>,
    audio: Res<Audio>,
    mut button_interactions: Query<
        (&Interaction, Option<&Disabled>),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, disabled) in &mut button_interactions {
        if !matches!(disabled, Some(Disabled(true))) {
            match *interaction {
                Interaction::Pressed => {
                    audio.play(audio_sources.button_press.clone());
                }
                Interaction::Hovered => {
                    audio.play(audio_sources.button_hover.clone());
                }
                Interaction::None => {}
            }
        }
    }
}

fn audio_construct(
    mut commands: Commands,
    audio_sources: Res<AudioSources>,
    audio: Res<Audio>,
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
                audio
                    .play(audio_sources.construct.clone())
                    .with_volume(0.35);
            }
            EntType::Road => {
                audio
                    .play(audio_sources.construct_road.clone())
                    .with_volume(0.35);
            }
            _ => {}
        }
    }
}

fn audio_constructed(
    mut commands: Commands,
    audio_sources: Res<AudioSources>,
    audio: Res<Audio>,
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
            audio
                .play(audio_sources.constructed.clone())
                .with_volume(0.25);

            // dont play audio for more than once entity
            // for this step ( overlapping sounds are loud )
            break;
        }
    }
}
