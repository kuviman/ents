use bevy::prelude::*;
use rand::Rng;

use crate::cursor;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_a_LOT_of_entities);
        app.add_systems(Update, hover);
        app.add_systems(Update, scale_hovered);
        app.add_systems(Update, click);
    }
}

#[derive(Component)]
struct Hovered;

fn hover(
    cursor: Query<&cursor::WorldPos>,
    entities: Query<(Entity, &GridCoords)>,
    mut commands: Commands,
) {
    let Ok(cursor) = cursor.get_single() else {
        return;
    };
    let cursor_grid_coords = cursor.0.floor().as_ivec2();
    for (entity, entity_coords) in entities.iter() {
        if entity_coords.0 == cursor_grid_coords {
            commands.entity(entity).try_insert(Hovered);
        } else {
            commands.entity(entity).remove::<Hovered>();
        }
    }
}

#[derive(Component)]
struct ScaleOnHover;

fn scale_hovered(mut entities: Query<(&mut Transform, Has<Hovered>), With<ScaleOnHover>>) {
    for (mut transform, hovered) in entities.iter_mut() {
        if hovered {
            transform.scale = Vec3::splat(1.5);
            transform.translation.z = 1.0;
        } else {
            transform.scale = Vec3::splat(1.0);
            transform.translation.z = 0.0;
        }
    }
}

fn click(
    input: Res<Input<MouseButton>>,
    hovered: Query<Entity, With<Hovered>>,
    mut commands: Commands,
) {
    if input.just_pressed(MouseButton::Left) {
        for entity in hovered.iter() {
            commands.entity(entity).despawn();
        }
    }
}

#[derive(Component)]
pub struct GridCoords(IVec2);

#[allow(non_snake_case)]
fn spawn_a_LOT_of_entities(mut commands: Commands) {
    const MAX_COORD: i32 = 100;

    let mut ents = Vec::new();

    for x in -MAX_COORD..=MAX_COORD {
        for y in -MAX_COORD..=MAX_COORD {
            ents.push((
                SpriteBundle {
                    sprite: Sprite {
                        color: Color::hsl(rand::thread_rng().gen_range(0.0..360.0), 0.5, 0.5),
                        custom_size: Some(Vec2::splat(1.0)),
                        ..default()
                    },
                    transform: Transform::from_translation(Vec3::new(
                        x as f32 + 0.5,
                        y as f32 + 0.5,
                        0.0,
                    )),
                    ..default()
                },
                GridCoords(IVec2::new(x, y)),
                ScaleOnHover,
            ));
        }
    }

    commands.spawn_batch(ents);

    commands.spawn({
        let mut camera = Camera2dBundle::new_with_far(1000.0);
        camera.projection.scaling_mode = bevy::render::camera::ScalingMode::FixedVertical(100.0);
        camera
    });
}
