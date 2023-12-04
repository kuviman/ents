use std::collections::VecDeque;

use bevy::{prelude::*, utils::HashMap};
use rand::{seq::SliceRandom, thread_rng, Rng};

use crate::{
    buttons,
    chunks::GeneratedChunks,
    cursor,
    tile_map::{Pos, Size, TileMap},
    ui,
};

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, generate_chunks);

        app.insert_resource(EntCosts({
            let mut costs = HashMap::new();
            costs.insert(EntType::Harvester, 5);
            costs.insert(EntType::Base, 7);
            costs
        }));

        app.add_systems(Startup, setup_ui);
        app.add_systems(Update, button_actions);
        app.add_systems(Update, disable_buttons);
        crate::buttons::register::<ButtonAction>(app);
        app.add_systems(Startup, setup_camera);
        // app.add_systems(Startup, spawn_a_LOT_of_entities);
        app.insert_resource(Money(0));
        app.add_systems(Update, update_money_text);
        app.add_systems(Update, scale_hovered);
        app.add_systems(
            Update,
            (hover_pixel, click_harvest).run_if(in_state(PlayerState::Normal)),
        );
        app.add_systems(
            Update,
            (place_ent, cancel_placing).run_if(|state: Res<State<PlayerState>>| {
                matches!(state.get(), PlayerState::Placing(..))
            }),
        );
        app.insert_resource(GlobalPathfinding {
            closest_harvest: default(),
        });
        app.add_systems(Update, (pathfind, ent_movement_to_harvest, ent_harvest));
        app.add_systems(Update, update_transforms);
        app.add_systems(Update, update_movement);
        app.add_state::<PlayerState>();
    }
}

fn generate_chunks(mut events: EventReader<crate::chunks::GenerateChunk>, mut commands: Commands) {
    let mut ents = Vec::new();

    for event in events.read() {
        let rect = event.rect();
        for x in rect.min.x..rect.max.x {
            for y in rect.min.y..rect.max.y {
                ents.push((
                    SpriteBundle {
                        sprite: Sprite {
                            color: Color::hsl(
                                thread_rng().gen_range({
                                    let off = 20.0;
                                    120.0 - off..120.0 + off
                                }),
                                0.7,
                                0.2,
                            ),
                            custom_size: Some(Vec2::splat(1.0)),
                            ..default()
                        },
                        ..default()
                    },
                    Pos(IVec2::new(x, y)),
                    ScaleOnHover,
                    Harvestable(1),
                ));
            }
        }
    }

    commands.spawn_batch(ents);
}

#[derive(Component)]
struct CanHavest;

#[derive(Component)]
struct Inventory {
    current: i32,
    max: i32,
}

#[derive(Component)]
struct Storing;

fn ent_harvest(
    mut ents: Query<
        (Entity, &Pos, &mut Inventory),
        (With<CanHavest>, With<Idle>, Without<Storing>),
    >,
    harvestables: Query<(Entity, &Harvestable)>,
    tile_map: Res<TileMap>,
    mut commands: Commands,
    mut money: ResMut<Money>,
) {
    for (ent, ent_pos, mut inventory) in ents.iter_mut() {
        let try_to_harvest = MOVE_DIRECTIONS
            .into_iter()
            .flat_map(|dir| tile_map.entities_at(ent_pos.0 + dir))
            .filter_map(|entity| harvestables.get(entity).ok())
            .next();
        if let Some((entity, harvestable)) = try_to_harvest {
            // TODO if inventory.current + harvestable.0 > inventory.max {
            //     commands.entity(ent).insert(Storing);
            // } else {
            commands.entity(entity).despawn();
            inventory.current += harvestable.0;
            money.0 += harvestable.0;
            // }
        }
    }
}

fn update_movement(
    mut q: Query<(Entity, &mut Pos, &mut Moving)>,
    time: Res<Time>,
    mut commands: Commands,
) {
    const ENT_MOVE_TIME: f32 = 0.2;
    for (entity, mut pos, mut moving) in q.iter_mut() {
        moving.t += time.delta_seconds() / ENT_MOVE_TIME;
        if moving.t > 1.0 {
            commands.entity(entity).remove::<Moving>().insert(Idle);
            pos.0 = moving.next_pos;
        }
    }
}

fn update_transforms(
    mut q: Query<
        (
            &mut Transform,
            &mut Sprite,
            &Pos,
            Option<&Size>,
            Option<&Moving>,
        ),
        Or<(Changed<Pos>, Changed<Moving>, Changed<Size>)>,
    >,
) {
    for (mut transform, mut sprite, pos, size, moving) in q.iter_mut() {
        let from = pos.0;
        let size = size.map_or(IVec2::splat(1), |size| size.0);
        let (to, t) = moving.map_or((from, 0.0), |moving| (moving.next_pos, moving.t));
        transform.translation = (from.as_vec2().lerp(to.as_vec2(), t) + size.as_vec2() / 2.0)
            .extend(transform.translation.z);
        sprite.custom_size = Some(size.as_vec2());
    }
}

#[derive(Copy, Clone, Debug)]
struct ClosestHarvest {
    distance: u32,
    ways: f64,
}

#[derive(Resource)]
struct GlobalPathfinding {
    closest_harvest: HashMap<IVec2, ClosestHarvest>,
}

const MOVE_DIRECTIONS: [IVec2; 4] = [IVec2::X, IVec2::Y, IVec2::NEG_X, IVec2::NEG_Y];

fn pathfind(
    mut pathfinding: ResMut<GlobalPathfinding>,
    harvestables: Query<&Pos, With<Harvestable>>,
    mut closest_harvest: Local<HashMap<IVec2, ClosestHarvest>>,
    generated_chunks: Res<GeneratedChunks>,
    mut q: Local<VecDeque<IVec2>>,
) {
    if q.is_empty() {
        pathfinding.closest_harvest = std::mem::take(&mut closest_harvest);
        for pos in harvestables.iter() {
            let pos = pos.0;
            closest_harvest.insert(
                pos,
                ClosestHarvest {
                    distance: 0,
                    ways: 1.0,
                },
            );
            q.push_back(pos);
        }
    }

    let mut iterations_left = 10000; // TODO base on time?
    while let Some(pos) = q.pop_front() {
        let current = *closest_harvest.get(&pos).unwrap();
        for dir in MOVE_DIRECTIONS {
            let next_pos = pos + dir;

            if !generated_chunks.is_generated(next_pos) {
                continue;
            }
            match closest_harvest.entry(next_pos) {
                bevy::utils::hashbrown::hash_map::Entry::Occupied(mut entry) => {
                    let that = entry.get_mut();
                    if that.distance == current.distance + 1 {
                        that.ways += current.ways;
                    }
                }
                bevy::utils::hashbrown::hash_map::Entry::Vacant(entry) => {
                    entry.insert(ClosestHarvest {
                        distance: current.distance + 1,
                        ways: current.ways,
                    });
                    q.push_back(next_pos);
                }
            }
        }

        iterations_left -= 1;
        if iterations_left == 0 {
            break;
        }
    }
}

#[derive(Component)]
struct Idle;

#[derive(Component)]
struct CanMove;

#[derive(Component)]
struct Moving {
    next_pos: IVec2,
    t: f32,
}

fn ent_movement_to_harvest(
    ents: Query<(Entity, &Pos), (With<CanMove>, With<Idle>, Without<Storing>)>,
    global_pathfinding: Res<GlobalPathfinding>,
    harvestables: Query<(), With<Harvestable>>,
    tile_map: Res<TileMap>,
    mut commands: Commands,
) {
    let local_pathfind = |from: IVec2| -> Option<IVec2> {
        let mut closest_harvest: HashMap<IVec2, ClosestHarvest> = default();
        let mut q = VecDeque::new();

        const RADIUS: i32 = 10;
        for x in from.x - RADIUS..=from.x + RADIUS {
            for y in from.y - RADIUS..=from.y + RADIUS {
                let pos = IVec2::new(x, y);
                for entity in tile_map.entities_at(pos) {
                    if harvestables.get(entity).is_ok() {
                        closest_harvest.insert(
                            pos,
                            ClosestHarvest {
                                distance: 0,
                                ways: 1.0,
                            },
                        );
                        q.push_back(pos);
                    }
                }
            }
        }

        while let Some(pos) = q.pop_front() {
            let current = *closest_harvest.get(&pos).unwrap();
            for dir in MOVE_DIRECTIONS {
                let next_pos: IVec2 = pos + dir;

                if (next_pos - from).abs().max_element() > RADIUS {
                    continue;
                }
                match closest_harvest.entry(next_pos) {
                    bevy::utils::hashbrown::hash_map::Entry::Occupied(mut entry) => {
                        let that = entry.get_mut();
                        if that.distance == current.distance + 1 {
                            that.ways += current.ways;
                        }
                    }
                    bevy::utils::hashbrown::hash_map::Entry::Vacant(entry) => {
                        entry.insert(ClosestHarvest {
                            distance: current.distance + 1,
                            ways: current.ways,
                        });
                        q.push_back(next_pos);
                    }
                }
            }
        }

        let from_closest_harvest = closest_harvest.get(&from)?;
        if from_closest_harvest.distance <= 1 {
            return None;
        }

        MOVE_DIRECTIONS
            .choose_weighted(&mut thread_rng(), |&dir| {
                closest_harvest.get(&(from + dir)).map_or(0.0, |h| {
                    if h.distance != from_closest_harvest.distance - 1 {
                        0.0
                    } else {
                        h.ways
                    }
                })
            })
            .ok()
            .copied()
    };

    let global_pathfind = |pos| {
        let closest_harvest = global_pathfinding.closest_harvest.get(&pos)?;
        if closest_harvest.distance <= 1 {
            return None;
        }
        MOVE_DIRECTIONS
            .choose_weighted(&mut thread_rng(), |&dir| {
                global_pathfinding
                    .closest_harvest
                    .get(&(pos + dir))
                    .map_or(0.0, |h| {
                        if h.distance != closest_harvest.distance - 1 {
                            0.0
                        } else {
                            h.ways
                        }
                    })
            })
            .ok()
            .copied()
    };

    for (entity, ent_pos) in ents.iter() {
        if let Some(dir) = local_pathfind(ent_pos.0).or_else(|| global_pathfind(ent_pos.0)) {
            commands
                .entity(entity)
                .insert(Moving {
                    next_pos: ent_pos.0 + dir,
                    t: 0.0,
                })
                .remove::<Idle>();
        }
    }
}

fn place_ent(
    cursor: Query<&cursor::WorldPos>,
    input: Res<Input<MouseButton>>,
    keyboard: Res<Input<KeyCode>>,
    mut commands: Commands,
    mut player_state: ResMut<NextState<PlayerState>>,
    mut money: ResMut<Money>,
    costs: Res<EntCosts>,
    state: Res<State<PlayerState>>,
) {
    let &PlayerState::Placing(ent_type) = state.get() else {
        unreachable!();
    };
    if input.just_pressed(MouseButton::Left) {
        let pos = cursor.single().0.floor().as_ivec2();

        // TODO check that empty

        money.0 -= costs.0[&ent_type];
        match ent_type {
            EntType::Harvester => {
                commands.spawn((
                    SpriteBundle {
                        sprite: Sprite {
                            color: Color::BLACK,
                            ..default()
                        },
                        ..default()
                    },
                    Pos(pos),
                    CanMove,
                    Inventory { current: 0, max: 1 },
                    Idle,
                    CanHavest,
                ));
            }
            EntType::Base => {
                commands.spawn((
                    SpriteBundle {
                        sprite: Sprite {
                            color: Color::RED,
                            ..default()
                        },
                        ..default()
                    },
                    Pos(pos),
                    Size(IVec2::splat(3)),
                    Idle,
                ));
            }
        }
        if !keyboard.pressed(KeyCode::ShiftLeft) {
            player_state.set(PlayerState::Normal);
        }
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
    Placing(EntType),
}

#[derive(Resource)]
struct EntCosts(HashMap<EntType, i32>);

fn disable_buttons(
    mut buttons: Query<(&mut buttons::Disabled, &ButtonAction)>,
    money: Res<Money>,
    costs: Res<EntCosts>,
) {
    for (mut disabled, action) in buttons.iter_mut() {
        disabled.0 = !match action {
            ButtonAction::Spawn(typ) => match costs.0.get(typ) {
                Some(&cost) => cost <= money.0,
                None => true,
            },
        };
    }
}

fn button_actions(
    mut events: EventReader<ButtonAction>,
    mut player_state: ResMut<NextState<PlayerState>>,
) {
    for event in events.read() {
        match event {
            &ButtonAction::Spawn(typ) => {
                player_state.set(PlayerState::Placing(typ));
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
enum EntType {
    Harvester,
    Base,
}

#[derive(Debug, Event, Component, Copy, Clone)]
enum ButtonAction {
    Spawn(EntType),
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
                for typ in [EntType::Harvester, EntType::Base] {
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
                            ButtonAction::Spawn(typ),
                            buttons::Disabled(false),
                        ))
                        .with_children(|button| {
                            button.spawn(TextBundle::from_section(format!("{typ:?}"), default()));
                        });
                }
            });
        });
}

#[derive(Resource)]
struct Money(i32);

#[derive(Component)]
struct Hovered;

fn hover_pixel(
    cursor: Query<&cursor::WorldPos>,
    hovered: Query<Entity, With<Hovered>>,
    tile_map: Res<TileMap>,
    ui_handling: Res<ui::UiHandling>,
    mut commands: Commands,
) {
    for entity in hovered.iter() {
        commands.entity(entity).remove::<Hovered>();
    }
    if ui_handling.is_pointer_over_ui {
        return;
    }
    let Ok(cursor) = cursor.get_single() else {
        return;
    };
    let cursor_pos = cursor.0.floor().as_ivec2();
    for entity in tile_map.entities_at(cursor_pos) {
        commands.entity(entity).try_insert(Hovered);
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
    hovered: Query<(Entity, &Harvestable), With<Hovered>>,
    mut money: ResMut<Money>,
    mut commands: Commands,
) {
    if input.just_pressed(MouseButton::Left) {
        for (entity, harvestable) in hovered.iter() {
            commands.entity(entity).despawn();
            money.0 += harvestable.0;
        }
    }
}

#[derive(Component)]
struct Harvestable(i32);

fn setup_camera(mut commands: Commands) {
    commands.spawn({
        let mut camera = Camera2dBundle::new_with_far(1000.0);
        camera.projection.scaling_mode = bevy::render::camera::ScalingMode::FixedVertical(100.0);
        (camera, UiCameraConfig { show_ui: true })
    });
}
