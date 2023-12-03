use bevy::prelude::*;

use crate::GameState;

pub struct ErrorPlugin;

impl Plugin for ErrorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Error), panic_system);
    }
}

fn panic_system() {
    panic!("We errored");
}
