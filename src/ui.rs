use bevy::prelude::*;

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(UiHandling {
            is_pointer_over_ui: false,
        });
        app.add_systems(
            Update,
            (
                ui_scale_because_ui_works_on_cameras_but_does_not_actually_use_cameras,
                check_ui_interaction,
            ),
        );
    }
}

fn ui_scale_because_ui_works_on_cameras_but_does_not_actually_use_cameras(
    window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut ui_scale: ResMut<UiScale>,
) {
    ui_scale.0 = window.single().height() as f64 / 500.0;
}

#[derive(Resource, Default)]
pub struct UiHandling {
    pub is_pointer_over_ui: bool,
}
#[derive(Component)]
pub struct NoPointerCapture;

fn check_ui_interaction(
    mut ui_handling: ResMut<UiHandling>,
    interaction_query: Query<&Interaction, (With<Node>, Without<NoPointerCapture>)>,
) {
    ui_handling.is_pointer_over_ui = interaction_query
        .iter()
        .any(|i| matches!(i, Interaction::Pressed | Interaction::Hovered));
}
