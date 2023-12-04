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

fn button_clicks<A: Copy + Component + Event>(
    buttons: Query<(Entity, &Interaction, &A), Changed<Interaction>>,
    mut prev_interaction: Local<HashMap<Entity, Interaction>>,
    mut click_events: EventWriter<A>,
) {
    for (button_entity, interaction, action) in buttons.iter() {
        if *interaction == Interaction::Hovered
            && prev_interaction.get(&button_entity) == Some(&Interaction::Pressed)
        {
            click_events.send(*action);
        }
        prev_interaction.insert(button_entity, *interaction);
    }
}

fn button_visuals(
    mut interaction_query: Query<
        (
            &Interaction,
            Option<&Disabled>,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        (Or<(Changed<Interaction>, Changed<Disabled>)>, With<Button>),
    >,
) {
    const NORMAL_BUTTON: Color = Color::rgb(0.15, 0.15, 0.15);
    const HOVERED_BUTTON: Color = Color::rgb(0.25, 0.25, 0.25);
    const PRESSED_BUTTON: Color = Color::rgb(0.35, 0.75, 0.35);
    const DISABLED_BUTTON: Color = Color::rgb(0.4, 0.4, 0.4);
    for (interaction, disabled, mut color, mut border_color) in &mut interaction_query {
        if disabled.map_or(false, |d| d.0) {
            *color = DISABLED_BUTTON.into();
            border_color.0 = Color::GRAY;
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
                border_color.0 = Color::BLACK;
            }
        }
    }
}
