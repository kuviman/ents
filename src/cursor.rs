use bevy::prelude::*;

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, cursor);
    }
}

#[derive(Component)]
pub struct WorldPos(pub Vec2);

fn cursor(
    window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    camera: Query<(Entity, &Camera, &GlobalTransform), With<Camera3d>>,
    mut commands: Commands,
) {
    let Some(cursor_window_pos) = window.single().cursor_position() else {
        return;
    };
    for (camera_entity, camera, camera_global_transform) in camera.iter() {
        if let Some(ray) = camera.viewport_to_world(camera_global_transform, cursor_window_pos) {
            let world_pos = (ray.origin + ray.direction * (-ray.origin.y / ray.direction.y)).xz();
            commands.entity(camera_entity).insert(WorldPos(world_pos));
        }
    }
}
