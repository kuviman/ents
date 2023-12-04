use bevy::prelude::*;

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            ui_scale_because_ui_works_on_cameras_but_does_not_actually_use_cameras,
        );
    }
}

fn ui_scale_because_ui_works_on_cameras_but_does_not_actually_use_cameras(
    window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut ui_scale: ResMut<UiScale>,
) {
    ui_scale.0 = window.single().height() as f64 / 500.0;
}
