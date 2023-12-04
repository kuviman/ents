use bevy::{input::mouse::MouseWheel, prelude::*};

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, camera_controls);
    }
}

fn camera_controls(
    keyboard: Res<Input<KeyCode>>,
    mouse_buttons: Res<Input<MouseButton>>,
    mut cursor_events: EventReader<CursorMoved>,
    mut wheel: EventReader<MouseWheel>,
    mut camera: Query<(
        &mut Transform,
        &GlobalTransform,
        &mut OrthographicProjection,
        &Camera,
    )>,
    time: Res<Time>,
    mut prev_cursor_pos: Local<Vec2>,
) {
    const CAMERA_SPEED: f32 = 50.0;

    let (mut camera_transform, global_camera_transform, mut projection, camera) =
        camera.single_mut();
    let mut dir = Vec2::ZERO;

    if keyboard.any_pressed([KeyCode::W, KeyCode::Up]) {
        dir.y += 1.0;
    }
    if keyboard.any_pressed([KeyCode::A, KeyCode::Left]) {
        dir.x -= 1.0;
    }
    if keyboard.any_pressed([KeyCode::S, KeyCode::Down]) {
        dir.y -= 1.0;
    }
    if keyboard.any_pressed([KeyCode::D, KeyCode::Right]) {
        dir.x += 1.0;
    }

    camera_transform.translation += dir.extend(0.0) * CAMERA_SPEED * time.delta_seconds();

    for wheel in wheel.read() {
        projection.scale = (projection.scale - wheel.y * 0.1).clamp(0.1, 2.0);
    }

    for moved in cursor_events.read() {
        if mouse_buttons.pressed(MouseButton::Middle) {
            let Some(prev_world_pos) =
                camera.viewport_to_world_2d(global_camera_transform, *prev_cursor_pos)
            else {
                continue;
            };
            let Some(new_world_pos) =
                camera.viewport_to_world_2d(global_camera_transform, moved.position)
            else {
                continue;
            };
            camera_transform.translation += (prev_world_pos - new_world_pos).extend(0.0);
        }
        *prev_cursor_pos = moved.position;
    }
}
