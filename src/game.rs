use bevy::prelude::*;
use rand::Rng;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_a_LOT_of_entities);
    }
}

#[allow(non_snake_case)]
fn spawn_a_LOT_of_entities(mut commands: Commands) {
    const MAX_COORD: i32 = 100;

    let mut ents = Vec::new();

    for x in -MAX_COORD..=MAX_COORD {
        for y in -MAX_COORD..=MAX_COORD {
            ents.push(SpriteBundle {
                sprite: Sprite {
                    color: Color::hsl(rand::thread_rng().gen_range(0.0..360.0), 0.5, 0.5),
                    custom_size: Some(Vec2::splat(1.0)),
                    anchor: bevy::sprite::Anchor::BottomLeft,
                    ..default()
                },
                transform: Transform::from_translation(Vec3::new(x as f32, y as f32, 0.0)),
                ..default()
            });
        }
    }

    commands.spawn_batch(ents);

    commands.spawn({
        let mut camera = Camera2dBundle::new_with_far(1000.0);
        camera.projection.scaling_mode = bevy::render::camera::ScalingMode::FixedVertical(100.0);
        camera
    });
}
