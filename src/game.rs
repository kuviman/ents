use std::marker::PhantomData;

use bevy::{
    prelude::*,
    utils::{HashMap, HashSet},
};
use rand::{seq::IteratorRandom, thread_rng, Rng};

use crate::{
    buttons, cursor,
    pathfind::{AppExt, Blocking, Pathfinding},
    tile_map::{Pos, Size, TileMap},
    ui,
};

pub const MOVE_DIRECTIONS: [IVec2; 4] = [IVec2::X, IVec2::Y, IVec2::NEG_X, IVec2::NEG_Y];

pub struct GamePlugin;

#[derive(Resource)]
struct Noise(noise::OpenSimplex);

impl Noise {
    fn get(&self, pos: Vec2) -> f32 {
        noise::NoiseFn::get(&self.0, [pos.x as f64, pos.y as f64]) as f32
    }
}

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Noise(noise::OpenSimplex::new(thread_rng().gen())));
        app.add_systems(Update, generate_chunks);

        app.add_systems(Update, harvestable_color);

        app.insert_resource(EntCosts({
            let mut costs = HashMap::new();
            // costs.insert(EntType::Harvester, 5);
            costs.insert(EntType::House, 10);
            costs.insert(EntType::Road, 1);
            costs.insert(EntType::UpgradeInventory, 50);
            costs.insert(EntType::Storage, 100);
            costs.insert(EntType::BuilderAcademy, 50);
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
            (place_ent.after(update_placing_preview), cancel_placing).run_if(
                |state: Res<State<PlayerState>>| matches!(state.get(), PlayerState::Placing(..)),
            ),
        );
        app.register_pathfinding_towards::<Harvestable>();
        app.register_pathfinding_towards::<StorageThatHasSpace>();
        app.add_systems(Update, (update_storages, visualize_storage));
        app.add_systems(
            Update,
            (
                ent_movement::<Harvesting, Harvestable>,
                ent_movement::<Storing, StorageThatHasSpace>,
                ent_harvest,
                ent_store,
            ),
        );
        app.add_systems(Update, update_transforms);
        app.add_systems(Update, update_movement);

        app.register_pathfinding_towards::<NonEmptyStorage>();
        app.register_pathfinding_towards::<NeedsResource>();
        app.add_systems(
            Update,
            (
                ent_movement::<TakingResource, NonEmptyStorage>,
                ent_movement::<BringingResource, NeedsResource>,
                take_resource,
                bring_resource,
                actual_building,
            ),
        );

        app.add_systems(Update, spawn_ents);
        app.add_systems(PostUpdate, ent_types);

        register_upgrade::<InventoryUpgrade>(app);
        register_upgrade::<BuilderUpgrade>(app);

        app.add_state::<PlayerState>();

        app.add_systems(
            Update,
            stop_placing_if_not_enough_money.run_if(|state: Res<State<PlayerState>>| {
                matches!(state.get(), PlayerState::Placing(..))
            }),
        );
        app.add_systems(Update, update_placing_preview);
    }
}

#[derive(Component)]
struct PlacementPreview;

#[derive(Component)]
struct PlacementBlocked(bool);

#[derive(Component)]
struct StorageThatHasSpace;

#[derive(Component)]
struct StorageLabel;

fn visualize_storage(
    mut text: Query<&mut Text, With<StorageLabel>>,
    storages: Query<(Entity, &Storage, Option<&Children>), Changed<Storage>>,
    mut commands: Commands,
) {
    for (entity, storage, children) in storages.iter() {
        let new_text = format!("{}/{}", storage.current, storage.max);
        if let Some(child) = children
            .map(|children| children.iter())
            .into_iter()
            .flatten()
            .copied()
            .find(|&child| text.get(child).is_ok())
        {
            let mut text = text.get_mut(child).unwrap();
            text.sections[0].value = new_text;
        } else {
            commands
                .spawn((
                    Text2dBundle {
                        text: Text::from_section(
                            new_text,
                            TextStyle {
                                font_size: 36.0,
                                color: Color::BLACK,
                                ..default()
                            },
                        ),
                        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 1.0))
                            .with_scale(Vec3::splat(0.03)),
                        ..default()
                    },
                    StorageLabel,
                ))
                .set_parent(entity);
        }
    }
}

#[derive(Component)]
struct NonEmptyStorage;

fn update_storages(
    q: Query<
        (
            Entity,
            &Storage,
            Has<StorageThatHasSpace>,
            Has<NonEmptyStorage>,
        ),
        Changed<Storage>,
    >,
    mut commands: Commands,
) {
    for (entity, storage, had_space, was_non_empty) in q.iter() {
        let has_space = storage.current < storage.max;
        let now_non_empty = storage.current != 0;
        if has_space != had_space {
            if has_space {
                commands.entity(entity).insert(StorageThatHasSpace);
            } else {
                commands.entity(entity).remove::<StorageThatHasSpace>();
            }
        }
        if was_non_empty != now_non_empty {
            if now_non_empty {
                commands.entity(entity).insert(NonEmptyStorage);
            } else {
                commands.entity(entity).remove::<NonEmptyStorage>();
            }
        }
    }
}

fn update_placing_preview(
    mut preview: Query<
        (
            &mut Pos,
            &mut Size,
            &mut Sprite,
            &mut Visibility,
            &mut PlacementBlocked,
        ),
        With<PlacementPreview>,
    >,
    roads: Query<(), With<Road>>,
    blocking: Query<(), With<Blocking>>,
    tile_map: Res<TileMap>,
    cursor: Query<&cursor::WorldPos>,
    state: Res<State<PlayerState>>,
    mut commands: Commands,
) {
    let ent_type = match state.get() {
        &PlayerState::Placing(ent_type) => Some(ent_type),
        _ => None,
    };
    match preview.get_single_mut() {
        Ok((mut pos, mut size, mut sprite, mut visibility, mut blocked)) => {
            if let Some(ent_type) = ent_type {
                let cell = cursor.single().0.floor().as_ivec2();
                pos.0 = cell;
                size.0 = ent_type.size();
                sprite.color = ent_type.color().with_a(0.5);

                let off = match ent_type {
                    EntType::Road => 0,
                    _ => 1,
                };

                let rect = IRect::from_corners(cell, cell + ent_type.size() - IVec2::splat(1));

                blocked.0 = (-off..ent_type.size().x + off)
                    .flat_map(|dx| {
                        (-off..ent_type.size().y + off).map(move |dy| cell + IVec2::new(dx, dy))
                    })
                    .any(|cell| {
                        tile_map.entities_at(cell).any(|entity| {
                            blocking.get(entity).is_ok()
                                || (rect.contains(cell) && roads.get(entity).is_ok())
                        })
                    });
                if blocked.0 {
                    sprite.color = Color::RED.with_a(0.5);
                    // size.0 += IVec2::splat(2);
                    // pos.0 -= IVec2::splat(1);
                };

                *visibility = Visibility::Visible;
            } else {
                blocked.0 = true;
                *visibility = Visibility::Hidden;
            }
        }
        Err(_) => {
            commands.spawn((
                SpriteBundle {
                    visibility: Visibility::Hidden,
                    transform: Transform::from_translation(Vec3::new(0.0, 0.0, 100.0)),
                    ..default()
                },
                Pos(IVec2::ZERO),
                Size(IVec2::splat(1)),
                PlacementPreview,
                PlacementBlocked(true),
            ));
        }
    }
}

fn stop_placing_if_not_enough_money(
    money: Res<Money>,
    costs: Res<EntCosts>,
    state: Res<State<PlayerState>>,
    mut next_state: ResMut<NextState<PlayerState>>,
) {
    let &PlayerState::Placing(ent_type) = state.get() else {
        return;
    };
    let Some(&ent_cost) = costs.0.get(&ent_type) else {
        return;
    };
    if money.0 < ent_cost {
        next_state.set(PlayerState::Normal);
    }
}

fn register_upgrade<U: Upgrade>(app: &mut App) {
    app.add_systems(
        Update,
        (
            start_assigning_upgrades::<U>,
            assign_upgrades::<U>,
            ent_movement::<GoingForUpgrade<U>, CanUpgrade<U>>,
            receive_upgrade::<U>,
        ),
    );
    app.register_pathfinding_towards::<CanUpgrade<U>>();
}

trait Upgrade: Component {
    fn new() -> Self;
    fn new_ent_type() -> EntType;
}

fn receive_upgrade<U: Upgrade>(
    ents: Query<(Entity, &Pos), With<GoingForUpgrade<U>>>,
    mut upgrade_shops: Query<&mut CanUpgrade<U>>,
    tile_map: Res<TileMap>,
    mut commands: Commands,
) {
    for (ent, ent_pos) in ents.iter() {
        for dir in MOVE_DIRECTIONS {
            let upgrade_shop = tile_map
                .entities_at(ent_pos.0 + dir)
                .find(|&entity| upgrade_shops.get(entity).is_ok());
            if let Some(upgrade_shop_entity) = upgrade_shop {
                let mut upgrade = upgrade_shops.get_mut(upgrade_shop_entity).unwrap();
                if upgrade.upgrades_left > 0 {
                    upgrade.upgrades_left -= 1;
                    commands.entity(ent).despawn();
                    commands.spawn((Pos(ent_pos.0), U::new_ent_type()));
                    // commands
                    //     .entity(ent)
                    //     .insert(U::new())
                    //     .remove::<(GoingForUpgrade<U>, GoingForAnyUpgrade)>()
                    //     .insert(Harvesting); // TODO maybe other?
                    if upgrade.upgrades_left == 0 {
                        commands
                            .entity(upgrade_shop_entity)
                            .remove::<CanUpgrade<U>>();
                    }
                }
                break;
            }
        }
    }
}

fn start_assigning_upgrades<U: Upgrade>(
    q: Query<(Entity, &CanUpgrade<U>), Added<CanUpgrade<U>>>,
    mut commands: Commands,
) {
    for (entity, can_upgrade) in q.iter() {
        commands.entity(entity).insert(NeedToAssignUpgrades::<U> {
            unassigned: can_upgrade.upgrades_left,
            phantom_data: PhantomData,
        });
    }
}
fn assign_upgrades<U: Upgrade>(
    ents: Query<
        Entity,
        (
            With<CanReceiveUpgrades>,
            Without<U>,
            Without<GoingForAnyUpgrade>,
        ),
    >,
    mut upgrade_shops: Query<(Entity, &mut NeedToAssignUpgrades<U>)>,
    mut commands: Commands,
) {
    let mut ents_to_upgrade = ents.iter();
    for (shop_entity, mut shops) in upgrade_shops.iter_mut() {
        if shops.unassigned == 0 {
            commands
                .entity(shop_entity)
                .remove::<NeedToAssignUpgrades<U>>();
        } else if let Some(ent) = ents_to_upgrade.next() {
            shops.unassigned -= 1;
            commands
                .entity(ent)
                .insert((GoingForAnyUpgrade, GoingForUpgrade::<U>(PhantomData)))
                .remove::<Harvesting>()
                .remove::<Storing>(); // TODO what if I have more state??
        } else {
            break;
        }
    }
}

#[derive(Component)]
struct CanReceiveUpgrades;

#[derive(Component)]
struct Spawn {
    ent_type: EntType,
    amount: usize,
}

#[derive(Component)]
struct Home(Entity);

fn spawn_ents(
    mut spawners: Query<(Entity, &Pos, Option<&Size>, &mut Spawn)>,
    mut commands: Commands,
) {
    for (spawner_entity, pos, size, mut spawn) in spawners.iter_mut() {
        if spawn.amount == 0 {
            commands.entity(spawner_entity).remove::<Spawn>();
        } else {
            let size = size.map_or(IVec2::splat(1), |size| size.0);

            let mut possible_spawn_locations = HashSet::new();
            for dx in 0..size.x {
                possible_spawn_locations.insert(pos.0 + IVec2::new(dx, 0));
                possible_spawn_locations.insert(pos.0 + IVec2::new(dx, size.y - 1));
            }
            for dy in 0..size.y {
                possible_spawn_locations.insert(pos.0 + IVec2::new(0, dy));
                possible_spawn_locations.insert(pos.0 + IVec2::new(size.x - 1, dy));
            }
            let spawn_pos = possible_spawn_locations
                .into_iter()
                .choose(&mut thread_rng())
                .unwrap();
            spawn.amount -= 1;
            commands.spawn((Pos(spawn_pos), Home(spawner_entity), spawn.ent_type));
        }
    }
}

#[derive(Component)]
struct ProvidePopulation(usize);

fn harvestable_color(mut q: Query<(&mut Sprite, &Harvestable), Changed<Harvestable>>) {
    for (mut sprite, harvestable) in q.iter_mut() {
        sprite
            .color
            .set_a(0.8 + 0.2 * (harvestable.0 as f32 / 10.0).min(1.0));
    }
}

#[derive(Component)]
struct CanUpgrade<T> {
    upgrades_left: usize,
    phantom_data: PhantomData<T>,
}

#[derive(Component)]
struct NeedToAssignUpgrades<T> {
    unassigned: usize,
    phantom_data: PhantomData<T>,
}

#[derive(Component)]
struct InventoryUpgrade;

impl Upgrade for InventoryUpgrade {
    fn new() -> Self {
        Self
    }
    fn new_ent_type() -> EntType {
        EntType::GoldHarvester
    }
}

#[derive(Component)]
struct BuilderUpgrade;

impl Upgrade for BuilderUpgrade {
    fn new() -> Self {
        Self
    }
    fn new_ent_type() -> EntType {
        EntType::Builder
    }
}

#[derive(Component)]
struct GoingForAnyUpgrade;

#[derive(Component)]
struct GoingForUpgrade<T>(PhantomData<T>);

#[derive(Component)]
struct CanBuild;

fn ent_types(q: Query<(Entity, &EntType), Added<EntType>>, mut commands: Commands) {
    for (entity, ent_type) in q.iter() {
        match ent_type {
            EntType::Storage => {
                commands.entity(entity).insert((
                    Storage {
                        current: 0,
                        max: 500,
                    },
                    Blocking,
                ));
            }
            EntType::Road => {
                commands.entity(entity).insert(Road);
            }
            EntType::Harvester => {
                commands.entity(entity).insert((
                    CanMove,
                    Inventory { current: 0, max: 1 },
                    Idle,
                    CanHavest,
                    UsesPopulation,
                    Harvesting,
                    CanReceiveUpgrades,
                ));
            }
            EntType::GoldHarvester => {
                commands.entity(entity).insert((
                    CanMove,
                    Inventory {
                        current: 0,
                        max: 10,
                    },
                    Idle,
                    CanHavest,
                    UsesPopulation,
                    Harvesting,
                ));
            }
            EntType::UpgradeInventory => {
                commands.entity(entity).insert((
                    Blocking,
                    CanUpgrade::<InventoryUpgrade> {
                        upgrades_left: 5,
                        phantom_data: PhantomData,
                    },
                ));
            }
            EntType::BuilderAcademy => {
                commands.entity(entity).insert((
                    Blocking,
                    CanUpgrade::<BuilderUpgrade> {
                        upgrades_left: 5,
                        phantom_data: PhantomData,
                    },
                ));
            }
            EntType::Base => {
                commands.entity(entity).insert((
                    Storage {
                        current: 0,
                        max: 1000,
                    },
                    Blocking,
                    ProvidePopulation(5),
                ));
            }
            EntType::House => {
                commands.entity(entity).insert((
                    Blocking,
                    ProvidePopulation(5),
                    Spawn {
                        ent_type: EntType::Harvester,
                        amount: 5,
                    },
                ));
            }
            EntType::Builder => {
                commands.entity(entity).insert((
                    CanMove,
                    Idle,
                    CanBuild,
                    Inventory { current: 0, max: 5 },
                    TakingResource,
                ));
            }
        }
        commands.entity(entity).insert((
            SpriteBundle {
                sprite: Sprite {
                    color: ent_type.color(),
                    ..default()
                },
                transform: Transform::from_translation(Vec3::new(0.0, 0.0, ent_type.z())),
                ..default()
            },
            Size(ent_type.size()),
        ));
    }
}

#[derive(Component)]
struct Storage {
    current: i32,
    max: i32,
}

fn generate_chunks(
    noise: Res<Noise>,
    mut events: EventReader<crate::chunks::GenerateChunk>,
    mut commands: Commands,
) {
    let mut pixels = Vec::new();

    for event in events.read() {
        let rect = event.rect();
        for x in rect.min.x..rect.max.x {
            for y in rect.min.y..rect.max.y {
                let pos = IVec2::new(x, y);
                if (pos.x == 0 || pos.y == 0) && pos.length_squared() == 25 {
                    commands.spawn((Pos(pos), EntType::Builder));
                }
                if pos == IVec2::ZERO - EntType::Base.size() / 2 {
                    commands.spawn((Pos(pos), EntType::Base));
                }
                if pos.length_squared() > 100 {
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
                        Pos(pos),
                        ScaleOnHover,
                        Harvestable(
                            (Vec2::new(x as f32, y as f32).length() / 20.0
                                + noise.get(pos.as_vec2() / 5.0) * 5.0)
                                .max(0.0) as i32
                                + 1,
                        ),
                        Blocking,
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
    mut storage: Query<&mut Storage>,
    tile_map: Res<TileMap>,
    mut money: ResMut<Money>,
    mut commands: Commands,
) {
    for (ent, ent_pos, mut inventory) in ents.iter_mut() {
        let try_to_store = MOVE_DIRECTIONS
            .into_iter()
            .flat_map(|dir| tile_map.entities_at(ent_pos.0 + dir))
            .find(|&entity| storage.get(entity).is_ok());
        if let Some(storage_entity) = try_to_store {
            let mut storage = storage.get_mut(storage_entity).unwrap();
            let amount_to_store = inventory.current.min(storage.max - storage.current);
            inventory.current -= amount_to_store;
            storage.current += amount_to_store;
            money.0 += amount_to_store;
            if inventory.current == 0 {
                commands.entity(ent).remove::<Storing>().insert(Harvesting);
            }
        }
    }
}

#[derive(Component)]
struct TakingResource;

#[derive(Component)]
struct BringingResource;

fn take_resource(
    mut ents: Query<(Entity, &Pos, &mut Inventory), (With<Idle>, With<TakingResource>)>,
    mut storage: Query<&mut Storage>,
    tile_map: Res<TileMap>,
    mut commands: Commands,
) {
    for (ent, ent_pos, mut inventory) in ents.iter_mut() {
        let try_to_store = MOVE_DIRECTIONS
            .into_iter()
            .flat_map(|dir| tile_map.entities_at(ent_pos.0 + dir))
            .find(|&entity| storage.get(entity).is_ok());
        if let Some(storage_entity) = try_to_store {
            let mut storage = storage.get_mut(storage_entity).unwrap();
            let amount_to_take = storage.current.min(inventory.max - inventory.current);
            inventory.current += amount_to_take;
            storage.current -= amount_to_take;
            if inventory.current == inventory.max {
                commands
                    .entity(ent)
                    .remove::<TakingResource>()
                    .insert(BringingResource);
            }
        }
    }
}

fn actual_building(
    query: Query<(Entity, &NeedsResource, &Pos, &Size, &Placeholder), Changed<NeedsResource>>,
    mut commands: Commands,
) {
    for (entity, needs, pos, size, placeholder) in query.iter() {
        if needs.0 == 0 {
            commands.entity(entity).despawn();
            commands.spawn((Pos(pos.0), Size(size.0), placeholder.0));
        }
    }
}

fn bring_resource(
    mut ents: Query<(Entity, &Pos, &mut Inventory), (With<Idle>, With<BringingResource>)>,
    mut needs: Query<&mut NeedsResource>,
    tile_map: Res<TileMap>,
    mut commands: Commands,
) {
    for (ent, ent_pos, mut inventory) in ents.iter_mut() {
        if inventory.current == 0 {
            commands
                .entity(ent)
                .remove::<BringingResource>()
                .insert(TakingResource);
            continue;
        }
        let placeholder = MOVE_DIRECTIONS
            .into_iter()
            .flat_map(|dir| tile_map.entities_at(ent_pos.0 + dir))
            .find(|&entity| needs.get(entity).is_ok());
        if let Some(placeholder) = placeholder {
            let mut need = needs.get_mut(placeholder).unwrap();
            let amount_to_bring = need.0.min(inventory.current);
            inventory.current -= amount_to_bring;
            need.0 -= amount_to_bring;
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

#[derive(Component)]
struct Road;

fn update_movement(
    mut q: Query<(Entity, &mut Pos, &mut Moving)>,
    time: Res<Time>,
    roads: Query<With<Road>>,
    tile_map: Res<TileMap>,
    mut commands: Commands,
) {
    for (entity, mut pos, mut moving) in q.iter_mut() {
        let move_time = if tile_map
            .entities_at(pos.0)
            .any(|entity| roads.get(entity).is_ok())
        {
            0.1
        } else {
            0.2
        };
        moving.t += time.delta_seconds() / move_time;
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

#[derive(Component)]
struct NeedsResource(i32);

#[derive(Component)]
struct Placeholder(EntType);

fn place_ent(
    input: Res<Input<MouseButton>>,
    mut commands: Commands,
    mut money: ResMut<Money>,
    preview: Query<(&Pos, &PlacementBlocked)>,
    costs: Res<EntCosts>,
    state: Res<State<PlayerState>>,
) {
    let &PlayerState::Placing(ent_type) = state.get() else {
        unreachable!();
    };
    let Ok((pos, blocked)) = preview.get_single() else {
        return;
    };

    if blocked.0 {
        return;
    }
    if input.just_pressed(MouseButton::Left) || input.pressed(MouseButton::Left) {
        let cost = costs.0[&ent_type];
        money.0 -= cost;
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: ent_type.color().with_a(0.5),
                    ..default()
                },
                ..default()
            },
            Blocking, // TODO
            Pos(pos.0),
            Size(ent_type.size()),
            Placeholder(ent_type),
            NeedsResource(cost),
        ));
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
    Storage,
    House,
    UpgradeInventory,
    Road,
    GoldHarvester,
    Builder,
    BuilderAcademy,
}

impl EntType {
    fn z(&self) -> f32 {
        match self {
            Self::Road => 0.0,
            _ => 1.0,
        }
    }
    fn color(&self) -> Color {
        match self {
            EntType::Harvester => Color::BLACK,
            EntType::Base => Color::RED,
            EntType::Storage => Color::BEIGE,
            EntType::House => Color::PURPLE,
            EntType::UpgradeInventory => Color::YELLOW,
            EntType::Road => Color::GRAY,
            EntType::GoldHarvester => Color::GOLD.with_l(0.1),
            EntType::Builder => Color::PINK.with_l(0.2),
            EntType::BuilderAcademy => Color::PINK,
        }
    }
    fn size(&self) -> IVec2 {
        match self {
            EntType::Storage => IVec2::new(4, 3),
            EntType::Base => IVec2::splat(5),
            EntType::House => IVec2::splat(2),
            EntType::UpgradeInventory => IVec2::new(2, 3),
            EntType::BuilderAcademy => IVec2::new(3, 2),
            _ => IVec2::splat(1),
        }
    }
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
                for typ in [
                    EntType::House,
                    EntType::Road,
                    EntType::BuilderAcademy,
                    EntType::UpgradeInventory,
                    EntType::Storage,
                ] {
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
    mut storage: Query<&mut Storage>,
    tile_map: Res<TileMap>,
    hovered: Query<(Entity, &Harvestable), With<Hovered>>,
    mut money: ResMut<Money>,
    mut commands: Commands,
) {
    if input.just_pressed(MouseButton::Left) {
        for (entity, harvestable) in hovered.iter() {
            for probably_most_likely_the_base in tile_map.entities_at(IVec2::ZERO) {
                if let Ok(mut storage) = storage.get_mut(probably_most_likely_the_base) {
                    storage.current += harvestable.0;
                }
            }
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
