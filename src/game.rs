use std::collections::HashMap;

use bevy::prelude::*;
use rand::Rng;

use crate::cursor;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_ui);
        app.add_systems(Update, button_visuals);
        app.add_systems(Update, button_actions);
        app.add_event::<ButtonAction>();
        app.add_systems(Update, button_clicks::<ButtonAction>);
        app.add_systems(
            Update,
            ui_scale_because_ui_works_on_cameras_but_does_not_actually_use_cameras,
        );
        app.add_systems(Startup, spawn_a_LOT_of_entities);
        app.insert_resource(Money(0));
        app.add_systems(Update, update_money_text);
        app.add_systems(Update, hover);
        app.add_systems(Update, scale_hovered);
        app.add_systems(Update, click);
    }
}

fn button_actions(mut events: EventReader<ButtonAction>) {
    for event in events.read() {
        info!("{event:?}");
    }
}

#[derive(Debug, Event, Component, Copy, Clone)]
enum ButtonAction {
    SpawnMinion,
}

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
        (&Interaction, &mut BackgroundColor, &mut BorderColor),
        (Changed<Interaction>, With<Button>),
    >,
) {
    const NORMAL_BUTTON: Color = Color::rgb(0.15, 0.15, 0.15);
    const HOVERED_BUTTON: Color = Color::rgb(0.25, 0.25, 0.25);
    const PRESSED_BUTTON: Color = Color::rgb(0.35, 0.75, 0.35);
    for (interaction, mut color, mut border_color) in &mut interaction_query {
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

fn update_money_text(mut money_text: Query<&mut Text, With<MoneyText>>, money: Res<Money>) {
    for mut money_text in money_text.iter_mut() {
        money_text.sections[0].value = format!("MONEY: {}", money.0);
    }
}

fn ui_scale_because_ui_works_on_cameras_but_does_not_actually_use_cameras(
    window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut ui_scale: ResMut<UiScale>,
) {
    ui_scale.0 = window.single().height() as f64 / 500.0;
}

#[derive(Component)]
struct MoneyText;

fn setup_ui(mut commands: Commands) {
    // commands.spawn({
    //     let mut camera = Camera2dBundle::default();
    //     camera.projection.scaling_mode = bevy::render::camera::ScalingMode::FixedVertical(10.0);
    //     camera
    // });
    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            ..default()
        })
        .with_children(|root| {
            root.spawn((TextBundle::from_section("$$$", default()), MoneyText));
            root.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    bottom: Val::Px(24.0),
                    position_type: PositionType::Absolute,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                ..default()
            })
            .with_children(|bottom| {
                bottom
                    .spawn((
                        ButtonBundle {
                            style: Style {
                                width: Val::Px(100.0),
                                height: Val::Px(40.0),
                                border: UiRect::all(Val::Px(5.0)),
                                // horizontally center child text
                                justify_content: JustifyContent::Center,
                                // vertically center child text
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            ..default()
                        },
                        ButtonAction::SpawnMinion,
                    ))
                    .with_children(|button| {
                        button.spawn(TextBundle::from_section("button", default()));
                    });
            });
        });
}

#[derive(Resource)]
struct Money(i32);

#[derive(Component)]
struct Hovered;

fn hover(
    cursor: Query<&cursor::WorldPos>,
    entities: Query<(Entity, &GridCoords)>,
    mut commands: Commands,
) {
    let Ok(cursor) = cursor.get_single() else {
        return;
    };
    let cursor_grid_coords = cursor.0.floor().as_ivec2();
    for (entity, entity_coords) in entities.iter() {
        if entity_coords.0 == cursor_grid_coords {
            commands.entity(entity).try_insert(Hovered);
        } else {
            commands.entity(entity).remove::<Hovered>();
        }
    }
}

#[derive(Component)]
struct ScaleOnHover;

fn scale_hovered(mut entities: Query<(&mut Transform, Has<Hovered>), With<ScaleOnHover>>) {
    for (mut transform, hovered) in entities.iter_mut() {
        if hovered {
            transform.scale = Vec3::splat(1.5);
            transform.translation.z = 1.0;
        } else {
            transform.scale = Vec3::splat(1.0);
            transform.translation.z = 0.0;
        }
    }
}

fn click(
    input: Res<Input<MouseButton>>,
    hovered: Query<Entity, With<Hovered>>,
    mut money: ResMut<Money>,
    mut commands: Commands,
) {
    if input.just_pressed(MouseButton::Left) {
        for entity in hovered.iter() {
            commands.entity(entity).despawn();
            money.0 += 1;
        }
    }
}

#[derive(Component)]
pub struct GridCoords(IVec2);

#[allow(non_snake_case)]
fn spawn_a_LOT_of_entities(mut commands: Commands) {
    const MAX_COORD: i32 = 100;

    let mut ents = Vec::new();

    for x in -MAX_COORD..=MAX_COORD {
        for y in -MAX_COORD..=MAX_COORD {
            ents.push((
                SpriteBundle {
                    sprite: Sprite {
                        color: Color::hsl(rand::thread_rng().gen_range(0.0..360.0), 0.5, 0.5),
                        custom_size: Some(Vec2::splat(1.0)),
                        ..default()
                    },
                    transform: Transform::from_translation(Vec3::new(
                        x as f32 + 0.5,
                        y as f32 + 0.5,
                        0.0,
                    )),
                    ..default()
                },
                GridCoords(IVec2::new(x, y)),
                ScaleOnHover,
            ));
        }
    }

    commands.spawn_batch(ents);

    commands.spawn({
        let mut camera = Camera2dBundle::new_with_far(1000.0);
        camera.projection.scaling_mode = bevy::render::camera::ScalingMode::FixedVertical(100.0);
        (camera, UiCameraConfig { show_ui: true })
    });
}
