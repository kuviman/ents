use bevy::prelude::*;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, empty_system);
    }
}

fn empty_system() {
    info!("WOW so empty");
}
