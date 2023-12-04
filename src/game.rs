use bevy::prelude::*;
use rand::Rng;

use crate::{buttons, cursor, ui};

pub struct GamePlugin;

const MINION_COST: i32 = 10;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_ui);
        app.add_systems(Update, button_actions);
        app.add_systems(Update, disable_buttons);
        crate::buttons::register::<ButtonAction>(app);
        app.add_systems(Startup, spawn_a_LOT_of_entities);
        app.insert_resource(Money(0));
        app.add_systems(Update, update_money_text);
        app.add_systems(Update, scale_hovered);
        app.add_systems(
            Update,
            (hover_pixel, click_harvest).run_if(in_state(PlayerState::Normal)),
        );
        app.add_systems(
            Update,
            (place_minion, cancel_placing).run_if(in_state(PlayerState::PlacingMinion)),
        );
        app.add_state::<PlayerState>();
    }
}

fn place_minion(
    cursor: Query<&cursor::WorldPos>,
    input: Res<Input<MouseButton>>,
    mut commands: Commands,
    mut player_state: ResMut<NextState<PlayerState>>,
    mut money: ResMut<Money>,
) {
    if input.just_pressed(MouseButton::Left) {
        let pos = cursor.single().0.floor().as_ivec2();

        // TODO check that empty

        money.0 -= MINION_COST;
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::hsl(rand::thread_rng().gen_range(0.0..360.0), 0.5, 0.5),
                    custom_size: Some(Vec2::splat(1.0)),
                    ..default()
                },
                transform: Transform::from_translation(Vec3::new(
                    pos.x as f32 + 0.5,
                    pos.y as f32 + 0.5,
                    0.0,
                )),
                ..default()
            },
            GridCoords(pos),
            ScaleOnHover,
        ));
        player_state.set(PlayerState::Normal);
    }
}

fn cancel_placing(
    input: Res<Input<MouseButton>>,
    mut player_state: ResMut<NextState<PlayerState>>,
) {
    if input.just_pressed(MouseButton::Right) {
        player_state.set(PlayerState::Normal);
    }
}

#[derive(States, Default, Debug, PartialEq, Eq, Hash, Clone)]
enum PlayerState {
    #[default]
    Normal,
    PlacingMinion,
}

fn disable_buttons(mut buttons: Query<(&mut buttons::Disabled, &ButtonAction)>, money: Res<Money>) {
    for (mut disabled, action) in buttons.iter_mut() {
        disabled.0 = action.cost() > money.0;
    }
}

fn button_actions(
    mut events: EventReader<ButtonAction>,
    mut player_state: ResMut<NextState<PlayerState>>,
) {
    for event in events.read() {
        match event {
            ButtonAction::SpawnMinion => {
                player_state.set(PlayerState::PlacingMinion);
            }
        }
    }
}

#[derive(Debug, Event, Component, Copy, Clone)]
enum ButtonAction {
    SpawnMinion,
}

impl ButtonAction {
    fn cost(&self) -> i32 {
        match self {
            ButtonAction::SpawnMinion => MINION_COST,
        }
    }
}

fn update_money_text(mut money_text: Query<&mut Text, With<MoneyText>>, money: Res<Money>) {
    for mut money_text in money_text.iter_mut() {
        money_text.sections[0].value = format!("MONEY: {}", money.0);
    }
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
                        buttons::Disabled(false),
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

fn hover_pixel(
    cursor: Query<&cursor::WorldPos>,
    entities: Query<(Entity, &GridCoords)>,
    ui_handling: Res<ui::UiHandling>,
    mut commands: Commands,
) {
    // TODO optimize
    let Ok(cursor) = cursor.get_single() else {
        return;
    };
    let cursor_grid_coords = cursor.0.floor().as_ivec2();
    for (entity, entity_coords) in entities.iter() {
        if entity_coords.0 == cursor_grid_coords && !ui_handling.is_pointer_over_ui {
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

fn click_harvest(
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
