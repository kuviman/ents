use std::collections::HashMap;

use bevy::prelude::*;

pub struct Plugin;

pub fn register<A: Event + Copy + Component>(app: &mut App) {
    app.add_event::<A>();
    app.add_systems(Update, button_clicks::<A>);
}

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, button_visuals);
    }
}

#[derive(Component)]
pub struct Disabled(pub bool);

#[derive(Component)]
pub struct Keybind(pub KeyCode);

fn button_clicks<A: Copy + Component + Event>(
    input: Res<Input<KeyCode>>,
    keybinds: Query<(&A, Option<&Disabled>, &Keybind)>,
    buttons: Query<(Entity, Option<&Disabled>, &Interaction, &A), Changed<Interaction>>,
    mut prev_interaction: Local<HashMap<Entity, Interaction>>,
    mut click_events: EventWriter<A>,
) {
    for (action, disabled, bind) in keybinds.iter() {
        if input.just_pressed(bind.0) && !disabled.map_or(false, |d| d.0) {
            click_events.send(*action);
        }
    }
    for (button_entity, disabled, interaction, action) in buttons.iter() {
        if *interaction == Interaction::Hovered
            && prev_interaction.get(&button_entity) == Some(&Interaction::Pressed)
            && !disabled.map_or(false, |d| d.0)
        {
            click_events.send(*action);
        }
        prev_interaction.insert(button_entity, *interaction);
    }
}

#[derive(Component)]
pub struct Active;

fn button_visuals(
    mut interaction_query: Query<
        (
            &Interaction,
            Has<Active>,
            Option<&Disabled>,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        (Or<(Changed<Interaction>, Changed<Disabled>)>, With<Button>),
    >,
) {
    const NORMAL_BUTTON: Color = Color::rgb(0.5, 0.5, 0.5);
    const HOVERED_BUTTON: Color = Color::rgb(0.7, 0.7, 0.7);
    const PRESSED_BUTTON: Color = Color::rgb(0.35, 0.75, 0.35);
    const DISABLED_BUTTON: Color = Color::rgb(0.2, 0.2, 0.2);
    for (interaction, active, disabled, mut color, mut border_color) in &mut interaction_query {
        if disabled.map_or(false, |d| d.0) {
            *color = DISABLED_BUTTON.into();
            border_color.0 = Color::BLACK;
            continue;
        }
        if active {
            *color = PRESSED_BUTTON.into();
            border_color.0 = Color::RED;
            continue;
        }
        match *interaction {
            Interaction::Pressed => {
                *color = PRESSED_BUTTON.into();
                border_color.0 = Color::RED;
            }
            Interaction::Hovered => {
                *color = HOVERED_BUTTON.into();
                border_color.0 = Color::WHITE;
            }
            Interaction::None => {
                *color = NORMAL_BUTTON.into();
                border_color.0 = Color::rgb(0.2, 0.2, 0.2);
            }
        }
    }
}
