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
    mut camera: Query<(&mut Transform, &GlobalTransform, &Camera, &mut Projection)>,
    time: Res<Time>,
    mut prev_cursor_pos: Local<Vec2>,
) {
    const CAMERA_SPEED: f32 = 50.0;

    let Ok((mut camera_transform, global_camera_transform, camera, mut projection)) =
        camera.get_single_mut()
    else {
        return;
    };
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

    let normalize = |v: Vec3| -> Vec2 { v.xz().normalize() };

    let dir = normalize(camera_transform.local_x()) * dir.x
        + normalize(camera_transform.local_y()) * dir.y;

    camera_transform.translation += dir.extend(0.0).xzy() * CAMERA_SPEED * time.delta_seconds();

    for wheel in wheel.read() {
        let Projection::Perspective(projection) = &mut *projection else {
            unreachable!()
        };
        projection.fov = (projection.fov
            - wheel.y
                * match wheel.unit {
                    bevy::input::mouse::MouseScrollUnit::Line => 10.0,
                    bevy::input::mouse::MouseScrollUnit::Pixel => 1.0,
                }
                * 1e-2)
            .clamp(5.0_f32.to_radians(), 60.0_f32.to_radians());
    }

    for moved in cursor_events.read() {
        if mouse_buttons.pressed(MouseButton::Middle) {
            let Some(prev_world_pos) = camera
                .viewport_to_world(global_camera_transform, *prev_cursor_pos)
                .map(|ray| (ray.origin - ray.direction * ray.origin.y / ray.direction.y).xz())
            else {
                continue;
            };
            let Some(new_world_pos) = camera
                .viewport_to_world(global_camera_transform, moved.position)
                .map(|ray| (ray.origin - ray.direction * ray.origin.y / ray.direction.y).xz())
            else {
                continue;
            };
            camera_transform.translation += (prev_world_pos - new_world_pos).extend(0.0).xzy();
        }
        *prev_cursor_pos = moved.position;
    }
}
