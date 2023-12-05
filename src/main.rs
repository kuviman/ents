use bevy::prelude::*;
use bevy::DefaultPlugins;

mod buttons;
mod camera_controls;
mod chunks;
mod cursor;
mod game;
mod pathfind;
mod tile_map;
mod ui;

fn main() {
    App::new()
        .insert_resource(Msaa::Off)
        .insert_resource(ClearColor(Color::rgb(0.4, 0.4, 0.4)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Ents".to_string(),
                // Bind to canvas included in `index.html`
                canvas: Some("#bevy".to_owned()),
                // The canvas size is constrained in index.html and build/web/styles.css
                fit_canvas_to_parent: true,
                // Tells wasm not to override default event handling, like F5 and Ctrl+R
                prevent_default_event_handling: false,
                ..default()
            }),
            ..default()
        }))
        .add_plugins((
            game::GamePlugin,
            cursor::Plugin,
            buttons::Plugin,
            ui::Plugin,
            chunks::Plugin,
            camera_controls::Plugin,
            tile_map::Plugin,
            pathfind::Plugin,
        ))
        .run();
}
