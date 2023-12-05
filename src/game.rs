use bevy::{prelude::*, utils::HashMap};
use rand::{thread_rng, Rng};

use crate::{
    buttons, cursor,
    pathfind::{AppExt, Pathfinding},
    tile_map::{Pos, Size, TileMap},
    ui,
};

pub const MOVE_DIRECTIONS: [IVec2; 4] = [IVec2::X, IVec2::Y, IVec2::NEG_X, IVec2::NEG_Y];

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, generate_chunks);

        app.insert_resource(EntCosts({
            let mut costs = HashMap::new();
            costs.insert(EntType::Harvester, 5);
            costs.insert(EntType::House, 30);
            costs
        }));

        app.add_systems(Startup, setup_ui);
        app.add_systems(Update, button_actions);
        app.add_systems(Update, disable_buttons);
        crate::buttons::register::<ButtonAction>(app);
        app.add_systems(Startup, setup_camera);
        // app.add_systems(Startup, spawn_a_LOT_of_entities);
        app.insert_resource(Money(0));
        app.add_systems(Update, (update_money_text, update_population_text));
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
        app.register_pathfinding_towards::<Harvestable>();
        app.register_pathfinding_towards::<Storage>();
        app.add_systems(
            Update,
            (
                ent_movement::<Harvesting, Harvestable>,
                ent_movement::<Storing, Storage>,
                ent_harvest,
                ent_store,
            ),
        );
        app.add_systems(Update, ent_types);
        app.add_systems(Update, update_transforms);
        app.add_systems(Update, update_movement);
        app.add_state::<PlayerState>();
    }
}

#[derive(Component)]
struct ProvidePopulation(usize);

fn ent_types(q: Query<(Entity, &EntType), Added<EntType>>, mut commands: Commands) {
    for (entity, ent_type) in q.iter() {
        match ent_type {
            EntType::Harvester => {
                commands.entity(entity).insert((
                    SpriteBundle {
                        sprite: Sprite {
                            color: Color::BLACK,
                            ..default()
                        },
                        ..default()
                    },
                    CanMove,
                    Inventory { current: 0, max: 1 },
                    Idle,
                    CanHavest,
                    UsesPopulation,
                    Harvesting,
                ));
            }
            EntType::Base => {
                commands.entity(entity).insert((
                    SpriteBundle {
                        sprite: Sprite {
                            color: Color::RED,
                            ..default()
                        },
                        ..default()
                    },
                    Size(IVec2::splat(3)),
                    Storage,
                    ProvidePopulation(5),
                ));
            }
            EntType::House => {
                commands.entity(entity).insert((
                    SpriteBundle {
                        sprite: Sprite {
                            color: Color::PURPLE,
                            ..default()
                        },
                        ..default()
                    },
                    Size(IVec2::splat(2)),
                    ProvidePopulation(5),
                ));
            }
        }
    }
}

#[derive(Component)]
struct Storage;

fn generate_chunks(mut events: EventReader<crate::chunks::GenerateChunk>, mut commands: Commands) {
    let mut pixels = Vec::new();

    for event in events.read() {
        let rect = event.rect();
        for x in rect.min.x..rect.max.x {
            for y in rect.min.y..rect.max.y {
                if x == -1 && y == -1 {
                    commands.spawn((Pos(IVec2::new(x, y)), EntType::Base));
                }
                if x * x + y * y > 10 {
                    pixels.push((
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
    }

    commands.spawn_batch(pixels);
}

#[derive(Component)]
struct CanHavest;

#[derive(Component)]
struct Inventory {
    current: i32,
    max: i32,
}

#[derive(Component)]
struct Harvesting;

#[derive(Component)]
struct Storing;

fn ent_store(
    mut ents: Query<(Entity, &Pos, &mut Inventory), (With<Idle>, With<Storing>)>,
    storage: Query<Entity, With<Storage>>,
    tile_map: Res<TileMap>,
    mut money: ResMut<Money>,
    mut commands: Commands,
) {
    for (ent, ent_pos, mut inventory) in ents.iter_mut() {
        let try_to_store = MOVE_DIRECTIONS
            .into_iter()
            .flat_map(|dir| tile_map.entities_at(ent_pos.0 + dir))
            .filter_map(|entity| storage.get(entity).ok())
            .next();
        if let Some(_storage_entity) = try_to_store {
            money.0 += inventory.current;
            inventory.current = 0;
            commands.entity(ent).remove::<Storing>().insert(Harvesting);
        }
    }
}

fn ent_harvest(
    mut ents: Query<
        (Entity, &Pos, &mut Inventory),
        (With<CanHavest>, With<Idle>, With<Harvesting>),
    >,
    mut harvestables: Query<(Entity, &mut Harvestable)>,
    tile_map: Res<TileMap>,
    mut commands: Commands,
) {
    for (ent, ent_pos, mut inventory) in ents.iter_mut() {
        let try_to_harvest = MOVE_DIRECTIONS
            .into_iter()
            .flat_map(|dir| tile_map.entities_at(ent_pos.0 + dir))
            .find(|&entity| harvestables.get(entity).is_ok());
        let try_to_harvest = try_to_harvest.map(|entity| harvestables.get_mut(entity).unwrap());
        if let Some((entity, mut harvestable)) = try_to_harvest {
            if harvestable.0 > 0 && inventory.current < inventory.max {
                harvestable.0 -= 1;
                inventory.current += 1;
                if harvestable.0 == 0 {
                    commands.entity(entity).despawn();
                    inventory.current += harvestable.0;
                }
            }
        }
        if inventory.current >= inventory.max {
            commands.entity(ent).insert(Storing).remove::<Harvesting>();
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
        Or<(
            Changed<Pos>,
            Changed<Moving>,
            Changed<Size>,
            Added<Sprite>,
            Added<Transform>,
        )>,
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

#[derive(Component)]
struct Idle;

#[derive(Component)]
struct CanMove;

#[derive(Component)]
struct Moving {
    next_pos: IVec2,
    t: f32,
}

fn ent_movement<EntState: Component, SearchingFor: Component>(
    ents: Query<(Entity, &Pos), (With<CanMove>, With<Idle>, With<EntState>)>,
    pathfinding: Res<Pathfinding<SearchingFor>>,
    mut commands: Commands,
) {
    for (entity, ent_pos) in ents.iter() {
        if let Some(dir) = pathfinding.pathfind(ent_pos.0) {
            if dir.distance > 1 {
                commands
                    .entity(entity)
                    .insert(Moving {
                        next_pos: ent_pos.0 + dir.dir,
                        t: 0.0,
                    })
                    .remove::<Idle>();
            }
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
        commands.spawn((Pos(pos), ent_type));
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

#[derive(Component)]
struct UsesPopulation;

fn disable_buttons(
    mut buttons: Query<(&mut buttons::Disabled, &ButtonAction)>,
    money: Res<Money>,
    costs: Res<EntCosts>,
    population_providers: Query<&ProvidePopulation>,
    population_users: Query<&UsesPopulation>,
) {
    let max_population: usize = population_providers
        .iter()
        .map(|population| population.0)
        .sum();
    let current_population = population_users.iter().count();
    for (mut disabled, action) in buttons.iter_mut() {
        match action {
            ButtonAction::Spawn(typ) => match costs.0.get(typ) {
                Some(&cost) => {
                    let has_money = cost <= money.0;

                    let need_population = match typ {
                        EntType::Harvester => 1,
                        _ => 0,
                    };
                    let has_population = need_population == 0
                        || current_population + need_population <= max_population;
                    disabled.0 = !(has_money && has_population);
                }
                None => disabled.0 = true,
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

#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash, Component)]
enum EntType {
    Harvester,
    Base,
    House,
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

fn update_population_text(
    mut text: Query<&mut Text, With<PopulationText>>,
    population_providers: Query<&ProvidePopulation>,
    population_users: Query<&UsesPopulation>,
) {
    let max: usize = population_providers
        .iter()
        .map(|population| population.0)
        .sum();
    let current = population_users.iter().count();
    for mut money_text in text.iter_mut() {
        money_text.sections[0].value = format!("POPULATION: {current}/{max}");
    }
}

#[derive(Component)]
struct PopulationText;

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
            root.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                ..default()
            })
            .with_children(|info| {
                info.spawn((TextBundle::from_section("$$$", default()), MoneyText));
                info.spawn((TextBundle::from_section("POP", default()), PopulationText));
            });
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
                for typ in [EntType::Harvester, EntType::House] {
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
