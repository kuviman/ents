use std::marker::PhantomData;

use bevy::{
    core_pipeline::tonemapping::Tonemapping,
    ecs::system::{EntityCommand, EntityCommands},
    prelude::*,
    render::{
        mesh::shape::Plane,
        texture::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor},
    },
    utils::{HashMap, HashSet},
    window::PrimaryWindow,
};
use rand::{seq::IteratorRandom, thread_rng, Rng};

const INITIAL_MONEY: i32 = 50;

use crate::{
    buttons, cursor, meshes,
    pathfind::{self, AppExt, Blocking, Pathfinding},
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

#[derive(Component)]
struct InventoryEntities(Vec<Entity>);

#[derive(Component)]
struct StorageLevelChild(Entity);

const BASE_HEIGHT: f32 = 1.0;

fn update_storage_visuals(
    storages: Query<
        (
            Option<&BuildingUpgradeComponent<Storage>>,
            &Storage,
            &StorageLevelChild,
        ),
        Changed<Storage>,
    >,
    mut levels: Query<&mut Transform>,
) {
    for (upgrade, storage, child) in storages.iter() {
        let mut child_transform = levels.get_mut(child.0).unwrap();
        child_transform.translation.y = storage.current as f32 / storage.max as f32
            * (upgrade.map_or(BASE_HEIGHT, |upgrade| {
                (upgrade.current_level + 1) as f32 * EntType::Storage.upgrade_height()
            }) - 0.1);
    }
}

#[derive(States, Default, Debug, PartialEq, Eq, Hash, Clone)]
pub enum WinState {
    #[default]
    NoWin,
    CrabRave,
}

fn start_crabrave(
    monuments: Query<&BuildingUpgradeComponent<MonumentUpgrade>, Without<NeedsResource>>,
    mut win_text: Query<&mut Style, With<WinText>>,
    mut next_state: ResMut<NextState<WinState>>,
) {
    if monuments.iter().any(|upgrade| upgrade.current_level == 3) {
        next_state.set(WinState::CrabRave);
        win_text.single_mut().display = Display::DEFAULT;
    }
}

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Noise(noise::OpenSimplex::new(thread_rng().gen())));
        app.add_systems(Update, generate_chunks);
        app.add_systems(Update, tooltip);

        app.add_systems(PostUpdate, start_crabrave.run_if(in_state(WinState::NoWin)));
        app.add_systems(Update, crabrave.run_if(in_state(WinState::CrabRave)));
        app.add_state::<WinState>();

        app.add_systems(Update, update_storage_visuals);

        app.insert_resource(EntCosts({
            let mut costs = HashMap::new();
            // costs.insert(EntType::Harvester, 5);
            costs.insert(EntType::House, 10);
            costs.insert(EntType::Road, 1);
            costs.insert(EntType::UpgradeInventory, 50);
            costs.insert(EntType::Storage, 100);
            costs.insert(EntType::BuilderAcademy, 50);
            costs.insert(EntType::Monument, 1000);
            costs
        }));

        app.add_systems(Startup, setup_ui);
        app.add_systems(Update, (unlock_buttons, button_actions));
        app.add_systems(Update, (activate_buttons, disable_buttons));
        crate::buttons::register::<ButtonAction>(app);
        app.add_systems(Startup, (setup_camera, setup_materials));
        // app.add_systems(Startup, spawn_a_LOT_of_entities);
        app.insert_resource(Money(INITIAL_MONEY));
        app.add_systems(
            Update,
            (
                update_money_text,
                update_population_text::<CanReceiveUpgrades, CrabsText>,
                update_population_text::<Gold, GoldText>,
                update_population_text::<CanBuild, BuildersText>,
            ),
        );
        app.add_systems(Update, scale_hovered);
        app.add_systems(Update, hovering.run_if(in_state(PlayerState::Normal)));
        app.add_systems(
            Update,
            (place_ent.after(update_placing_preview), cancel_placing).run_if(
                |state: Res<State<PlayerState>>| matches!(state.get(), PlayerState::Placing(..)),
            ),
        );
        app.add_systems(Update, scaffolding);
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
        app.add_systems(Update, inventory_entities);
        app.add_systems(Update, (update_transforms, update_resource_transforms));
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
        app.add_systems(Update, update_time_text.run_if(in_state(WinState::NoWin)));

        app.add_systems(
            Update,
            stop_placing_if_not_enough_money.run_if(|state: Res<State<PlayerState>>| {
                matches!(state.get(), PlayerState::Placing(..))
            }),
        );
        app.insert_resource(HavePlaced(false));
        app.add_systems(Update, stop_placing_on_mouse_release);
        app.add_systems(Update, update_placing_preview);
        app.add_systems(Update, bavy_monument);

        register_building_upgrade::<Storage>(app);
        register_building_upgrade::<ProvidePopulation>(app);
        register_building_upgrade::<MonumentUpgrade>(app);
    }
}

#[derive(Component)]
struct BavyBirds(Vec<Entity>);

#[derive(Component)]
struct BavyBird(f32);

fn bavy_monument(
    ent_materials: Res<EntMaterials>,
    mut q: Query<
        (
            Entity,
            &mut BavyBirds,
            &BuildingUpgradeComponent<MonumentUpgrade>,
        ),
        Without<NeedsResource>,
        // Changed<BuildingUpgradeComponent<MonumentUpgrade>>,
    >,
    mut birds: Query<(&mut Transform, &BavyBird)>,
    mut commands: Commands,
    time: Res<Time>,
) {
    for (entity, mut birds, upgrades) in q.iter_mut() {
        let birds = &mut birds.0;
        while birds.len() < upgrades.current_level as usize {
            birds.push(
                commands
                    .spawn((
                        PbrBundle {
                            mesh: ent_materials.bavy_mesh.clone(),
                            material: ent_materials
                                .bavy_materials
                                .get(birds.len())
                                .cloned()
                                .unwrap_or_default(),
                            transform: Transform::from_xyz(0.0, birds.len() as f32 + 1.0, 0.0),
                            ..default()
                        },
                        BavyBird(birds.len() as f32 + 1.0),
                    ))
                    .set_parent(entity)
                    .id(),
            );
        }
    }
    for (mut transform, bird) in birds.iter_mut() {
        transform.rotation = Quat::from_rotation_y(bird.0 * time.elapsed_seconds() * 0.3);
    }
}

impl BuildingUpgrade for ProvidePopulation {
    fn add_systems(app: &mut App) {
        app.add_systems(Update, upgrade_houses);
    }
    const BASE_COST: i32 = 20;
}

fn update_resource_transforms(mut q: Query<(&mut Transform, &Harvestable), Changed<Harvestable>>) {
    for (mut transform, harvestable) in q.iter_mut() {
        transform.translation.y = harvestable.0 as f32 - 1.0
    }
}

fn inventory_entities(
    ent_materials: Res<EntMaterials>,
    mut ents: Query<(Entity, &mut InventoryEntities, &Inventory), Changed<Inventory>>,
    mut commands: Commands,
) {
    for (ent, mut stack, inv) in ents.iter_mut() {
        let stack = &mut stack.0;
        while stack.len() > inv.current as _ {
            commands.entity(stack.pop().unwrap()).despawn();
        }
        while stack.len() < inv.current as _ {
            stack.push(
                commands
                    .spawn(PbrBundle {
                        mesh: ent_materials.inventory_thing_mesh.clone(),
                        material: ent_materials
                            .inventory_thing_material
                            .get(stack.len())
                            .cloned()
                            .unwrap_or_default(),
                        transform: Transform::from_xyz(0.0, (stack.len() + 1) as f32 * 0.1, 0.0)
                            .with_rotation(Quat::from_rotation_y(
                                thread_rng().gen_range(0.0..2.0 * std::f32::consts::PI),
                            )),
                        ..default()
                    })
                    .set_parent(ent)
                    .id(),
            );
        }
    }
}

fn scaffolding(
    q: Query<(Entity, &Size), (Added<NeedsResource>, Without<GhostRoad>)>,
    mut removed: RemovedComponents<NeedsResource>,
    ent_materials: Res<EntMaterials>,
    mut scaffolds: Local<HashMap<Entity, Entity>>,
    mut commands: Commands,
) {
    for (entity, size) in q.iter() {
        info!("SCAFF");
        let child = commands
            .spawn(PbrBundle {
                mesh: ent_materials.scaffold_mesh.clone(),
                material: ent_materials.scaffold_material.clone(),
                transform: Transform::from_scale(
                    (size.0.as_vec2() + Vec2::splat(0.2))
                        .extend(size.0.max_element() as f32)
                        .xzy(),
                ),
                ..default()
            })
            .set_parent(entity)
            .id();
        let old = scaffolds.insert(entity, child);
        assert!(old.is_none());
    }
    for entity in removed.read() {
        if let Some(child) = scaffolds.remove(&entity) {
            commands.entity(child).despawn();
        }
    }
}

fn upgrade_houses(
    mut events: EventReader<BuildingUpgradeEvent<ProvidePopulation>>,
    mut houses: Query<Option<&mut Spawn>>,
    mut commands: Commands,
) {
    for event in events.read() {
        if let Ok(Some(mut spawn)) = houses.get_mut(event.entity) {
            spawn.amount += 5;
        } else {
            commands.entity(event.entity).insert(Spawn {
                ent_type: EntType::Harvester,
                amount: 5,
            });
        }
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
            &mut Handle<Mesh>,
            &mut Handle<StandardMaterial>,
            &mut Visibility,
            &mut PlacementBlocked,
        ),
        With<PlacementPreview>,
    >,
    ent_materials: Res<EntMaterials>,
    roads: Query<(), Or<(With<GhostRoad>, With<Road>)>>,
    blocking: Query<(), Or<(With<Blocking>, With<BlockingGhost>)>>,
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
        Ok((mut pos, mut size, mut mesh, mut material, mut visibility, mut blocked)) => {
            if let Some(ent_type) = ent_type {
                let cell = cursor.single().0.floor().as_ivec2() - ent_type.size() / 2;
                pos.0 = cell;
                size.0 = ent_type.size();
                *mesh = ent_materials
                    .meshes
                    .get(&ent_type)
                    .cloned()
                    .unwrap_or_default();

                let rect = IRect::from_corners(cell, cell + ent_type.size() - IVec2::splat(1));

                fn iterate_rect(rect: IRect) -> impl Iterator<Item = IVec2> {
                    (rect.min.x..=rect.max.x)
                        .flat_map(move |x| (rect.min.y..=rect.max.y).map(move |y| IVec2::new(x, y)))
                }

                let is_blocking = |cell| {
                    tile_map
                        .entities_at(cell)
                        .any(|entity| blocking.get(entity).is_ok())
                };

                let is_road = |cell| {
                    tile_map
                        .entities_at(cell)
                        .any(|entity| roads.get(entity).is_ok())
                };

                blocked.0 = !match ent_type {
                    EntType::Road => {
                        !iterate_rect(rect).any(|cell| is_blocking(cell) || is_road(cell))
                            && iterate_rect(rect.inset(1))
                                .filter(|&cell| !rect.contains(cell))
                                .any(is_road)
                    }
                    _ => {
                        !iterate_rect(rect).any(is_road)
                            && !iterate_rect(rect.inset(1)).any(is_blocking)
                            && iterate_rect(rect.inset(1))
                                .filter(|&cell| !rect.contains(cell))
                                .any(is_road)
                    }
                };
                *material = ent_materials
                    .materials
                    .get(&(
                        ent_type,
                        if blocked.0 {
                            EntState::BlockedPreview
                        } else {
                            EntState::Preview
                        },
                    ))
                    .cloned()
                    .unwrap_or_default();

                *visibility = Visibility::Visible;
            } else {
                blocked.0 = true;
                *visibility = Visibility::Hidden;
            }
        }
        Err(_) => {
            commands.spawn((
                MaterialMeshBundle::<StandardMaterial> {
                    visibility: Visibility::Hidden,
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

#[derive(Resource)]
struct HavePlaced(bool);

fn init_have_placed(mut placed: ResMut<HavePlaced>) {
    placed.0 = false;
}

fn stop_placing_on_mouse_release(
    placed: Res<HavePlaced>,
    mouse: Res<Input<MouseButton>>,
    keyboard: Res<Input<KeyCode>>,
    mut next_state: ResMut<NextState<PlayerState>>,
) {
    if mouse.just_released(MouseButton::Left) && placed.0 && !keyboard.pressed(KeyCode::ShiftLeft) {
        next_state.set(PlayerState::Normal);
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
    register_building_upgrade::<U>(app);
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
                    commands.entity(ent).despawn_recursive();
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

impl<U: Upgrade> BuildingUpgrade for U {
    fn add_systems(app: &mut App) {
        app.add_systems(Update, assign_more_upgrades::<U>);
    }
    const BASE_COST: i32 = 100;
}

struct InsertOrModify<C> {
    f: Box<dyn FnOnce(&mut C) + Send>,
    default_value: C,
}

trait EntityCommandsExt {
    fn insert_or_modify<C: Component>(
        &mut self,
        default_value: C,
        f: impl Fn(&mut C) + Send + 'static,
    ) -> &mut Self;
}

impl EntityCommandsExt for EntityCommands<'_, '_, '_> {
    fn insert_or_modify<C: Component>(
        &mut self,
        default_value: C,
        f: impl FnOnce(&mut C) + Send + 'static,
    ) -> &mut Self {
        self.add(InsertOrModify {
            f: Box::new(f),
            default_value,
        })
    }
}

impl<C: Component> EntityCommand for InsertOrModify<C> {
    fn apply(self, id: Entity, world: &mut World) {
        let mut entity = world.entity_mut(id);
        if let Some(mut existing) = entity.get_mut() {
            (self.f)(&mut existing);
        } else {
            entity.insert(self.default_value);
        }
    }
}

fn assign_more_upgrades<U: Upgrade>(
    mut events: EventReader<BuildingUpgradeEvent<U>>,
    mut commands: Commands,
) {
    for event in events.read() {
        commands.entity(event.entity).insert_or_modify(
            NeedToAssignUpgrades::<U> {
                unassigned: 5,
                phantom_data: PhantomData,
            },
            |existing| existing.unassigned += 5,
        );
        commands.entity(event.entity).insert_or_modify(
            CanUpgrade::<U> {
                upgrades_left: 5,
                phantom_data: PhantomData,
            },
            |existing| existing.upgrades_left += 5,
        );
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

#[derive(Component)]
pub struct BuildingUpgradeComponent<T> {
    pub current_level: i32,
    pub phantom_data: PhantomData<T>,
}

impl<T> BuildingUpgradeComponent<T> {
    fn new() -> Self {
        Self {
            current_level: 0,
            phantom_data: PhantomData,
        }
    }
}

pub trait BuildingUpgrade: Send + Sync + 'static {
    fn add_systems(app: &mut App);
    const BASE_COST: i32;
}

fn register_building_upgrade<T: BuildingUpgrade>(app: &mut App) {
    app.add_systems(Update, tooltip_upgrade::<T>);
    app.add_systems(Update, make_hoverable::<T>);
    app.add_systems(Update, perform_building_upgrades::<T>);
    app.add_systems(PostUpdate, click_to_upgrade_building::<T>);
    app.add_systems(Update, update_upgrade_transforms::<T>);
    app.add_event::<BuildingUpgradeEvent<T>>();
    T::add_systems(app);
}

fn tooltip_upgrade<T: BuildingUpgrade>(
    mut q: Query<&mut Text, With<Tooltip>>,
    hovered: Query<&BuildingUpgradeComponent<T>, (With<Hovered>, Without<NeedsResource>)>,
) {
    if let Some(upgrade) = hovered.iter().next() {
        let cost = (upgrade.current_level + 1) * T::BASE_COST;
        q.single_mut().sections[0].value = cost.to_string();
    }
}

fn make_hoverable<T: BuildingUpgrade>(
    q: Query<Entity, Added<BuildingUpgradeComponent<T>>>,
    mut commands: Commands,
) {
    for entity in q.iter() {
        commands.entity(entity).insert(ScaleOnHover);
    }
}

#[derive(Component)]
pub struct BuildingUpgradeToPerform<T>(PhantomData<T>);

fn perform_building_upgrades<T: BuildingUpgrade>(
    buildings: Query<
        (
            Entity,
            &EntType,
            &BuildingUpgradeComponent<T>,
            &NeedsResource,
        ),
        (Changed<NeedsResource>, With<BuildingUpgradeToPerform<T>>),
    >,
    mut commands: Commands,
    mut events: EventWriter<BuildingUpgradeEvent<T>>,
) {
    for (entity, ent_type, upgrade, needs) in buildings.iter() {
        if needs.0 == 0 {
            commands.entity(entity).remove::<NeedsResource>();
            events.send(BuildingUpgradeEvent {
                entity,
                phantom_data: PhantomData,
            });
            if upgrade.current_level >= ent_type.max_upgrades() as _ {
                commands.entity(entity).remove::<ScaleOnHover>();
            }
        }
    }
}

fn click_to_upgrade_building<T: BuildingUpgrade>(
    input: Res<Input<MouseButton>>,
    mut buildings: Query<
        (Entity, &EntType, &mut BuildingUpgradeComponent<T>),
        (Without<NeedsResource>, With<Hovered>),
    >,
    mut money: ResMut<Money>,
    mut commands: Commands,
) {
    if !input.just_pressed(MouseButton::Left) {
        return;
    }
    let Some((building, ent_type, mut upgrades)) = buildings.iter_mut().next() else {
        return;
    };
    if upgrades.current_level >= ent_type.max_upgrades() as _ {
        return;
    }
    let cost = (upgrades.current_level + 1) * T::BASE_COST;
    if money.0 < cost {
        return;
    }
    money.0 -= cost;
    commands.entity(building).insert((
        NeedsResource(cost, cost),
        BuildingUpgradeToPerform::<T>(PhantomData),
    ));
    upgrades.current_level += 1;
}

#[derive(Event)]
struct BuildingUpgradeEvent<T> {
    entity: Entity,
    phantom_data: PhantomData<T>,
}

impl BuildingUpgrade for Storage {
    fn add_systems(app: &mut App) {
        app.add_systems(Update, building_upgrade_storage);
    }
    const BASE_COST: i32 = 200;
}

fn building_upgrade_storage(
    mut events: EventReader<BuildingUpgradeEvent<Storage>>,
    mut entities: Query<&mut Storage>,
) {
    for event in events.read() {
        if let Ok(mut storage) = entities.get_mut(event.entity) {
            storage.max += 500;
        }
    }
}

struct MonumentUpgrade;

impl BuildingUpgrade for MonumentUpgrade {
    fn add_systems(_app: &mut App) {}
    const BASE_COST: i32 = 1000;
}

fn ent_types(
    q: Query<(Entity, &Pos, &EntType), Added<EntType>>,
    ent_materials: Res<EntMaterials>,
    mut commands: Commands,
) {
    for (entity, pos, ent_type) in q.iter() {
        match ent_type {
            EntType::Monument => {
                commands.entity(entity).insert((
                    Blocking,
                    {
                        let mut up = BuildingUpgradeComponent::<MonumentUpgrade>::new();
                        up.current_level = 0;
                        up
                    },
                    BavyBirds(vec![]),
                ));
            }
            EntType::Storage => {
                let level = commands
                    .spawn(PbrBundle {
                        mesh: ent_materials.level_mesh.clone(),
                        material: ent_materials.level_material.clone(),
                        transform: Transform::from_scale(Vec3::new(
                            ent_type.size().x as f32,
                            1.0,
                            ent_type.size().y as f32,
                        ))
                        .with_translation(
                            (pos.0.as_vec2() + ent_type.size().as_vec2() / 2.0)
                                .extend(0.0)
                                .xzy(),
                        ),
                        ..default()
                    })
                    .id();
                commands.entity(entity).insert((
                    Storage {
                        current: 0,
                        max: 50,
                    },
                    Blocking,
                    BuildingUpgradeComponent::<Storage>::new(),
                    StorageLevelChild(level),
                ));
            }
            EntType::Road => {
                commands.entity(entity).insert(Road);
            }
            EntType::Harvester => {
                commands.entity(entity).insert((
                    CanMove,
                    Inventory { current: 0, max: 1 },
                    InventoryEntities(vec![]),
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
                    InventoryEntities(vec![]),
                    Idle,
                    CanHavest,
                    UsesPopulation,
                    Harvesting,
                    Gold,
                ));
            }
            EntType::UpgradeInventory => {
                commands.entity(entity).insert((
                    Blocking,
                    CanUpgrade::<InventoryUpgrade> {
                        upgrades_left: 5,
                        phantom_data: PhantomData,
                    },
                    BuildingUpgradeComponent::<InventoryUpgrade>::new(),
                ));
            }
            EntType::BuilderAcademy => {
                commands.entity(entity).insert((
                    Blocking,
                    CanUpgrade::<BuilderUpgrade> {
                        upgrades_left: 5,
                        phantom_data: PhantomData,
                    },
                    BuildingUpgradeComponent::<BuilderUpgrade>::new(),
                ));
            }
            EntType::Base => {
                let level = commands
                    .spawn(PbrBundle {
                        mesh: ent_materials.level_mesh.clone(),
                        material: ent_materials.level_material.clone(),
                        transform: Transform::from_scale(Vec3::new(
                            ent_type.size().x as f32,
                            1.0,
                            ent_type.size().y as f32,
                        ))
                        .with_translation(
                            (pos.0.as_vec2() + ent_type.size().as_vec2() / 2.0)
                                .extend(0.0)
                                .xzy(),
                        ),
                        ..default()
                    })
                    .id();
                commands.entity(entity).insert((
                    Storage {
                        current: INITIAL_MONEY,
                        max: 100,
                    },
                    Blocking,
                    ProvidePopulation(5),
                    StorageLevelChild(level),
                    Road,
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
                    BuildingUpgradeComponent::<ProvidePopulation>::new(),
                ));
            }
            EntType::Builder => {
                commands.entity(entity).insert((
                    CanMove,
                    Idle,
                    CanBuild,
                    Inventory { current: 0, max: 5 },
                    InventoryEntities(vec![]),
                    TakingResource,
                ));
            }
        }
        commands.entity(entity).insert((
            MaterialMeshBundle {
                mesh: ent_materials
                    .meshes
                    .get(ent_type)
                    .cloned()
                    .unwrap_or_default(),
                material: ent_materials
                    .materials
                    .get(&(*ent_type, EntState::Normal))
                    .cloned()
                    .unwrap_or_default(),
                transform: Transform::from_xyz(0.0, ent_type.height(), 0.0),
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
    ent_materials: Res<EntMaterials>,
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
                        MaterialMeshBundle {
                            // TODO?
                            // color: Color::hsl(
                            //     thread_rng().gen_range({
                            //         let off = 20.0;
                            //         120.0 - off..120.0 + off
                            //     }),
                            //     0.7,
                            //     0.2,
                            // ),
                            mesh: ent_materials.harvestable_mesh.clone(),
                            material: ent_materials.harvestable_material.clone(),
                            transform: Transform::from_rotation(Quat::from_rotation_y(
                                thread_rng().gen_range(0.0..2.0 * std::f32::consts::PI),
                            )),
                            ..default()
                        },
                        Pos(pos),
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
        for storage_entity in MOVE_DIRECTIONS
            .into_iter()
            .flat_map(|dir| tile_map.entities_at(ent_pos.0 + dir))
        {
            let Ok(mut storage) = storage.get_mut(storage_entity) else {
                continue;
            };
            let amount_to_store = inventory.current.min(storage.max - storage.current).max(0);
            inventory.current -= amount_to_store;
            storage.current += amount_to_store;
            money.0 += amount_to_store;
            if inventory.current == 0 {
                commands.entity(ent).remove::<Storing>().insert(Harvesting);
                break;
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
        for storage_entity in MOVE_DIRECTIONS
            .into_iter()
            .flat_map(|dir| tile_map.entities_at(ent_pos.0 + dir))
        {
            let Ok(mut storage) = storage.get_mut(storage_entity) else {
                continue;
            };
            let amount_to_take = storage
                .current
                .min(inventory.max - inventory.current)
                .max(0);
            inventory.current += amount_to_take;
            storage.current -= amount_to_take;
            if inventory.current == inventory.max {
                commands
                    .entity(ent)
                    .remove::<TakingResource>()
                    .insert(BringingResource);
                break;
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

#[derive(Component)]
struct GhostRoad;

#[derive(Component)]
pub struct BlockingGhost;

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
            commands.entity(entity).remove::<Moving>().try_insert(Idle);
            pos.0 = moving.next_pos;
        }
    }
}

fn update_upgrade_transforms<U: BuildingUpgrade>(
    mut q: Query<
        (
            &mut Transform,
            &EntType,
            Option<&NeedsResource>,
            &BuildingUpgradeComponent<U>,
        ),
        (
            // TODO ? Or<(Changed<NeedsResource>, Changed<BuildingUpgradeComponent<U>>)>,
            Without<BuildingUpgradeComponent<MonumentUpgrade>>,
        ),
    >,
) {
    for (mut transform, ent_type, needs, upgrade) in q.iter_mut() {
        transform.translation.y =
            ent_type.height() + upgrade.current_level as f32 * ent_type.upgrade_height();
        if let Some(needs) = needs {
            transform.translation.y -= ent_type.upgrade_height() * needs.0 as f32 / needs.1 as f32;
        }
    }
}

fn update_transforms(
    mut q: Query<
        (&mut Transform, &Pos, Option<&Size>, Option<&Moving>),
        Or<(
            Changed<Pos>,
            Changed<Moving>,
            Changed<Size>,
            Added<Transform>,
        )>,
    >,
    time: Res<Time>,
) {
    for (mut transform, pos, size, moving) in q.iter_mut() {
        let from = pos.0;
        let size = size.map_or(IVec2::splat(1), |size| size.0);
        let (to, t) = moving.map_or((from, 0.0), |moving| (moving.next_pos, moving.t));
        transform.translation = (from.as_vec2().lerp(to.as_vec2(), t) + size.as_vec2() / 2.0)
            .extend(transform.translation.y)
            .xzy();
        if let Some(moving) = moving {
            let delta = moving.next_pos - pos.0;
            if delta != IVec2::ZERO {
                transform.rotation = transform.rotation.lerp(
                    Quat::from_rotation_y(delta.as_vec2().angle_between(Vec2::X)),
                    (time.delta_seconds() * 15.0).min(1.0),
                );
            }
        }
    }
}

#[derive(Component)]
struct Idle;

#[derive(Component)]
pub struct CanMove;

#[derive(Component)]
pub struct Moving {
    pub next_pos: IVec2,
    pub t: f32,
}

fn crabrave(mut crabs: Query<&mut Transform, With<CanMove>>, time: Res<Time>) {
    for mut transform in crabs.iter_mut() {
        transform.rotation = Quat::from_rotation_y(
            (time.elapsed_seconds() * 10.0).sin() * std::f32::consts::PI / 4.0,
        );
    }
}

fn ent_movement<EntState: Component, SearchingFor: Component>(
    win_state: Res<State<WinState>>,
    pathfind_ents: Res<pathfind::Ents>,
    ents: Query<(Entity, &Pos), (With<CanMove>, With<Idle>, With<EntState>)>,
    blocking: Query<(&Pos, &Size), With<Blocking>>,
    tile_map: Res<TileMap>,
    pathfinding: Res<Pathfinding<SearchingFor>>,
    mut commands: Commands,
) {
    if matches!(win_state.get(), WinState::CrabRave) {
        return;
    }
    for (entity, ent_pos) in ents.iter() {
        if let Some(dir) = pathfinding.pathfind(&pathfind_ents, ent_pos.0) {
            if dir.distance > 1 {
                commands
                    .entity(entity)
                    .try_insert(Moving {
                        next_pos: ent_pos.0 + dir.dir,
                        t: 0.0,
                    })
                    .remove::<Idle>();
            }
        } else if let Some((pos, size)) = tile_map
            .entities_at(ent_pos.0)
            .find_map(|entity| blocking.get(entity).ok())
        {
            let dx = match (ent_pos.0.x - 1 - pos.0.x).cmp(&(pos.0.x + size.0.x - ent_pos.0.x)) {
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Greater => 1,
            };
            let dy = match (ent_pos.0.y - 1 - pos.0.y).cmp(&(pos.0.y + size.0.y - ent_pos.0.y)) {
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Greater => 1,
            };
            if dx != 0 || dy != 0 {
                commands
                    .entity(entity)
                    .try_insert(Moving {
                        next_pos: ent_pos.0 + IVec2::new(dx, dy),
                        t: 0.0,
                    })
                    .remove::<Idle>();
            }
        }
    }
}

#[derive(Component)]
pub struct NeedsResource(i32, i32);

#[derive(Component)]
pub struct Placeholder(pub EntType);

fn place_ent(
    input: Res<Input<MouseButton>>,
    ent_materials: Res<EntMaterials>,
    mut commands: Commands,
    mut money: ResMut<Money>,
    preview: Query<(&Pos, &PlacementBlocked)>,
    costs: Res<EntCosts>,
    state: Res<State<PlayerState>>,
    mut placed: ResMut<HavePlaced>,
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
        placed.0 = true;
        let cost = costs.0[&ent_type];
        money.0 -= cost;
        let mut entity = commands.spawn((
            MaterialMeshBundle {
                mesh: ent_materials
                    .meshes
                    .get(&ent_type)
                    .cloned()
                    .unwrap_or_default(),
                material: ent_materials
                    .materials
                    .get(&(ent_type, EntState::Placeholder))
                    .cloned()
                    .unwrap_or_default(),
                ..default()
            },
            Pos(pos.0),
            Size(ent_type.size()),
            Placeholder(ent_type),
            NeedsResource(cost, cost),
        ));
        if let EntType::Road = ent_type {
            entity.insert(GhostRoad);
        } else {
            entity.insert(BlockingGhost);
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

fn activate_buttons(
    buttons: Query<(Entity, &ButtonAction)>,
    state: Res<State<PlayerState>>,
    mut commands: Commands,
) {
    if state.is_changed() {
        for (entity, action) in buttons.iter() {
            let active = match state.get() {
                PlayerState::Placing(typ) => Some(typ),
                _ => None,
            } == match action {
                ButtonAction::Spawn(typ) => Some(typ),
            };
            if active {
                commands.entity(entity).insert(buttons::Active);
            } else {
                commands.entity(entity).remove::<buttons::Active>();
            }
        }
    }
}

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
    current_state: Res<State<PlayerState>>,
    mut player_state: ResMut<NextState<PlayerState>>,
    mut commands: Commands,
) {
    for event in events.read() {
        match event {
            &ButtonAction::Spawn(typ) => {
                if *current_state.get() != PlayerState::Placing(typ) {
                    player_state.set(PlayerState::Placing(typ));
                    commands.insert_resource(HavePlaced(false));
                } else {
                    player_state.set(PlayerState::Normal);
                }
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash, Component)]
pub enum EntType {
    Harvester,
    Base,
    Storage,
    House,
    UpgradeInventory,
    Road,
    GoldHarvester,
    Builder,
    BuilderAcademy,
    Monument,
}

impl EntType {
    fn all() -> impl Iterator<Item = Self> {
        [
            Self::Harvester,
            Self::Base,
            Self::Storage,
            Self::House,
            Self::UpgradeInventory,
            Self::Road,
            Self::GoldHarvester,
            Self::Builder,
            Self::BuilderAcademy,
            Self::Monument,
        ]
        .into_iter()
    }
    fn color(&self) -> Color {
        match self {
            EntType::Harvester => Color::BLACK,
            EntType::Base => Color::WHITE,
            EntType::Storage => Color::BEIGE,
            EntType::House => Color::PURPLE,
            EntType::UpgradeInventory => Color::YELLOW,
            EntType::Road => Color::GRAY,
            EntType::GoldHarvester => Color::GOLD.with_l(0.1),
            EntType::Builder => Color::PINK.with_l(0.2),
            EntType::BuilderAcademy => Color::WHITE,
            EntType::Monument => Color::AQUAMARINE,
        }
    }
    fn size(&self) -> IVec2 {
        match self {
            EntType::Storage => IVec2::new(4, 3),
            EntType::Base => IVec2::splat(5),
            EntType::House => IVec2::splat(2),
            EntType::UpgradeInventory => IVec2::new(2, 3),
            EntType::BuilderAcademy => IVec2::new(3, 2),
            EntType::Monument => IVec2::splat(10),
            _ => IVec2::splat(1),
        }
    }

    fn height(&self) -> f32 {
        match self {
            EntType::Harvester | EntType::GoldHarvester | EntType::Builder => 0.1,
            _ => 0.0,
        }
    }

    fn upgrade_height(&self) -> f32 {
        match self {
            EntType::House => 1.0,
            _ => 1.0,
        }
    }

    pub fn max_upgrades(&self) -> usize {
        match self {
            EntType::House => 9,
            EntType::BuilderAcademy | EntType::UpgradeInventory => 4,
            EntType::Storage => 4,
            EntType::Monument => 3,
            _ => 0,
        }
    }
}

#[derive(Debug, Event, Component, Copy, Clone)]
enum ButtonAction {
    Spawn(EntType),
}

fn update_money_text(mut money_text: Query<&mut Text, With<MoneyText>>, money: Res<Money>) {
    for mut money_text in money_text.iter_mut() {
        money_text.sections[0].value = format!("{}", money.0);
    }
}

#[derive(Component)]
struct Gold;

fn update_population_text<Filter: Component, TextFilter: Component>(
    mut crabs_text: Query<&mut Text, With<TextFilter>>,
    crabs: Query<(), With<Filter>>,
) {
    crabs_text.single_mut().sections[0].value = crabs.iter().len().to_string();
}

#[derive(Component)]
struct CrabsText;

#[derive(Component)]
struct BuildersText;

#[derive(Component)]
struct GoldText;

#[derive(Component)]
struct MoneyText;

#[derive(Component)]
struct Dependencies(HashSet<EntType>);

fn unlock_buttons(
    new_entities: Query<&EntType, Added<EntType>>,
    mut buttons: Query<(Entity, &mut Dependencies, &mut Style)>,
    mut commands: Commands,
) {
    for ent_type in new_entities.iter() {
        for (button_entity, mut deps, mut style) in buttons.iter_mut() {
            deps.0.remove(ent_type);
            if deps.0.is_empty() {
                commands.entity(button_entity).remove::<Dependencies>();
                style.display = default();
            }
        }
    }
}

#[derive(Component)]
struct Tooltip;

#[derive(Component)]
struct TimeText;

fn update_time_text(mut q: Query<&mut Text, With<TimeText>>, time: Res<Time>) {
    let seconds = time.elapsed_seconds() as i32;
    let minutes = seconds / 60;
    let seconds = seconds % 60;
    let hours = minutes / 60;
    let minutes = minutes % 60;
    q.single_mut().sections[0].value = format!("{}:{:02}:{:02}", hours, minutes, seconds);
}

fn tooltip(
    mut q: Query<(&mut Text, &mut Style), With<Tooltip>>,
    window: Query<&Window, With<PrimaryWindow>>,
    ui_scale: Res<UiScale>,
    hovered: Query<(), (With<Hovered>, With<ScaleOnHover>, Without<NeedsResource>)>,
    buttons: Query<(&ButtonAction, &Interaction)>,
    costs: Res<EntCosts>,
) {
    let (mut text, mut style) = q.single_mut();
    let Some(mut pos) = window.single().cursor_position() else {
        return;
    };
    pos.y = window.single().height() - pos.y;
    let pos = pos / ui_scale.0 as f32;
    style.left = Val::Px(pos.x);
    style.bottom = Val::Px(pos.y);

    style.display = Display::None;
    if hovered.iter().next().is_some() {
        style.display = default();
    } else if let Some(cost) = buttons.iter().find_map(|(action, interaction)| {
        if let Interaction::Hovered = interaction {
            match action {
                ButtonAction::Spawn(typ) => costs.0.get(typ).copied(),
            }
        } else {
            None
        }
    }) {
        text.sections[0].value = cost.to_string();
        style.display = default();
    }
}

#[derive(Component)]
struct WinText;

fn setup_ui(asset_server: Res<AssetServer>, mut commands: Commands) {
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
            root.spawn((
                TextBundle {
                    z_index: ZIndex::Global(10000),
                    style: Style {
                        position_type: PositionType::Absolute,
                        border: UiRect::all(Val::Px(5.0)),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::GRAY),
                    text: Text::from_section(
                        "tooltip",
                        TextStyle {
                            font_size: 32.0,
                            color: Color::BLACK,
                            ..default()
                        },
                    ),
                    ..default()
                },
                Outline {
                    width: Val::Px(3.0),
                    color: Color::DARK_GRAY,
                    offset: Val::ZERO,
                },
                Tooltip,
            ));
            let text_style = TextStyle {
                font_size: 32.0,
                color: Color::BLACK,
                ..default()
            };
            root.spawn(NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    right: Val::ZERO,
                    top: Val::ZERO,
                    ..default()
                },
                ..default()
            })
            .with_children(|info| {
                info.spawn(NodeBundle::default()).with_children(|time| {
                    time.spawn((TextBundle::from_section("0", text_style.clone()), TimeText));
                    time.spawn(ImageBundle {
                        image: UiImage::new(asset_server.load("icons/time.png")),
                        style: Style {
                            margin: UiRect {
                                left: Val::Px(10.0),
                                ..default()
                            },
                            ..default()
                        },
                        ..default()
                    });
                });
            });
            root.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                ..default()
            })
            .with_children(|info| {
                info.spawn(NodeBundle::default()).with_children(|money| {
                    money.spawn(ImageBundle {
                        image: UiImage::new(asset_server.load("icons/money.png")),
                        style: Style {
                            margin: UiRect {
                                right: Val::Px(10.0),
                                ..default()
                            },
                            ..default()
                        },
                        ..default()
                    });
                    money.spawn((
                        TextBundle::from_section("$$$", text_style.clone()),
                        MoneyText,
                    ));
                });
                info.spawn(NodeBundle::default()).with_children(|crabs| {
                    crabs.spawn(ImageBundle {
                        image: UiImage::new(asset_server.load("icons/crab.png")),
                        style: Style {
                            margin: UiRect {
                                right: Val::Px(10.0),
                                ..default()
                            },
                            ..default()
                        },
                        ..default()
                    });
                    crabs.spawn((
                        TextBundle::from_section("crabs", text_style.clone()),
                        CrabsText,
                    ));
                });
                info.spawn(NodeBundle::default()).with_children(|crabs| {
                    crabs.spawn(ImageBundle {
                        image: UiImage::new(asset_server.load("icons/builders.png")),
                        style: Style {
                            margin: UiRect {
                                right: Val::Px(10.0),
                                ..default()
                            },
                            ..default()
                        },
                        ..default()
                    });
                    crabs.spawn((
                        TextBundle::from_section("bulders", text_style.clone()),
                        BuildersText,
                    ));
                });
                info.spawn(NodeBundle::default()).with_children(|crabs| {
                    crabs.spawn(ImageBundle {
                        image: UiImage::new(asset_server.load("icons/gold.png")),
                        style: Style {
                            margin: UiRect {
                                right: Val::Px(10.0),
                                ..default()
                            },
                            ..default()
                        },
                        ..default()
                    });
                    crabs.spawn((
                        TextBundle::from_section("gold", text_style.clone()),
                        GoldText,
                    ));
                });
            });
            root.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    top: Val::Px(24.0),
                    position_type: PositionType::Absolute,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                ..default()
            })
            .with_children(|top| {
                top.spawn((
                    TextBundle {
                        text: Text::from_section("YOU WIN", text_style.clone()),
                        style: Style {
                            display: Display::None,
                            ..default()
                        },
                        ..default()
                    },
                    WinText,
                ));
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
                for (typ, key, deps) in [
                    (EntType::Road, KeyCode::Key1, vec![]),
                    (EntType::House, KeyCode::Key2, vec![EntType::Road]),
                    (EntType::BuilderAcademy, KeyCode::Key3, vec![EntType::House]),
                    (
                        EntType::UpgradeInventory,
                        KeyCode::Key4,
                        vec![EntType::House],
                    ),
                    (EntType::Storage, KeyCode::Key5, vec![EntType::House]),
                    (
                        EntType::Monument,
                        KeyCode::Key6,
                        vec![
                            EntType::BuilderAcademy,
                            EntType::UpgradeInventory,
                            EntType::Storage,
                        ],
                    ),
                ] {
                    bottom
                        .spawn((
                            ButtonBundle {
                                style: Style {
                                    width: Val::Px(60.0),
                                    height: Val::Px(60.0),
                                    border: UiRect::all(Val::Px(5.0)),
                                    margin: UiRect::all(Val::Px(5.0)),
                                    // horizontally center child text
                                    justify_content: JustifyContent::Center,
                                    // vertically center child text
                                    align_items: AlignItems::Center,
                                    display: if deps.is_empty() {
                                        default()
                                    } else {
                                        Display::None
                                    },
                                    ..default()
                                },
                                ..default()
                            },
                            Dependencies(deps.into_iter().collect()),
                            ButtonAction::Spawn(typ),
                            buttons::Keybind(key),
                            buttons::Disabled(false),
                        ))
                        .with_children(|button| {
                            button.spawn(ImageBundle {
                                image: UiImage::new(asset_server.load(format!(
                                    "icons/{}.png",
                                    match typ {
                                        EntType::House => "house",
                                        EntType::BuilderAcademy => "builders",
                                        EntType::Monument => "bavy",
                                        EntType::Road => "road",
                                        EntType::Storage => "storage",
                                        EntType::UpgradeInventory => "gold",
                                        _ => unreachable!(),
                                    }
                                ))),
                                ..default()
                            });
                            // button.spawn(TextBundle::from_section(format!("{typ:?}"), default()));
                        });
                }
            });
        });
}

#[derive(Resource)]
pub struct Money(pub i32);

#[derive(Component)]
pub struct Hovered;

#[derive(Component)]
pub struct IsHovered(bool);

fn hovering(
    cursor: Query<&cursor::WorldPos>,
    hovered: Query<Entity, With<Hovered>>,
    tile_map: Res<TileMap>,
    ui_handling: Res<ui::UiHandling>,
    mut commands: Commands,
) {
    for entity in hovered.iter() {
        commands
            .entity(entity)
            .remove::<Hovered>()
            .try_insert(IsHovered(false));
    }
    if ui_handling.is_pointer_over_ui {
        return;
    }
    let Ok(cursor) = cursor.get_single() else {
        return;
    };
    let cursor_pos = cursor.0.floor().as_ivec2();
    for entity in tile_map.entities_at(cursor_pos) {
        commands
            .entity(entity)
            .try_insert((Hovered, IsHovered(true)));
    }
}

#[derive(Component)]
pub struct ScaleOnHover;

fn scale_hovered(
    mut entities: Query<
        (&mut Transform, &EntType, &IsHovered, Has<NeedsResource>),
        (With<ScaleOnHover>, Changed<IsHovered>),
    >,
) {
    for (mut transform, ent_type, hovered, needs) in entities.iter_mut() {
        if hovered.0 && !needs {
            let size = ent_type.size().max_element() as f32;
            transform.scale = Vec3::splat((size + 0.5) / size);
        } else {
            transform.scale = Vec3::splat(1.0);
        }
    }
}

#[derive(Component)]
struct Harvestable(i32);

#[derive(PartialEq, Eq, Hash)]
pub enum EntState {
    Placeholder,
    Normal,
    Hovered,
    BlockedPreview,
    Preview,
}

#[derive(Resource)]
struct EntMaterials {
    meshes: HashMap<EntType, Handle<Mesh>>,
    materials: HashMap<(EntType, EntState), Handle<StandardMaterial>>,
    harvestable_mesh: Handle<Mesh>,
    harvestable_material: Handle<StandardMaterial>,
    inventory_thing_mesh: Handle<Mesh>,
    inventory_thing_material: Vec<Handle<StandardMaterial>>,
    bavy_mesh: Handle<Mesh>,
    bavy_materials: Vec<Handle<StandardMaterial>>,
    level_mesh: Handle<Mesh>,
    level_material: Handle<StandardMaterial>,
    scaffold_mesh: Handle<Mesh>,
    scaffold_material: Handle<StandardMaterial>,
}

fn setup_materials(
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    let mut meshes = HashMap::new();
    let mut materials = HashMap::new();
    for ent_type in EntType::all() {
        meshes.insert(
            ent_type,
            // mesh_assets.add(Mesh::from(Quad::new(ent_type.size().as_vec2()))),
            mesh_assets.add(match ent_type {
                EntType::Builder | EntType::GoldHarvester | EntType::Harvester => {
                    Mesh::from(Plane::from_size(0.75))
                }
                EntType::Road | EntType::Monument => {
                    Mesh::from(Plane::from_size(ent_type.size().max_element() as f32))
                }
                _ => meshes::building_mesh(
                    ent_type.size(),
                    ent_type.upgrade_height(),
                    ent_type.max_upgrades() + 1,
                ),
            }),
        );
        let material = StandardMaterial {
            fog_enabled: true,
            perceptual_roughness: 1.0,
            metallic: 0.0,
            reflectance: 0.0,
            alpha_mode: match ent_type {
                EntType::Builder
                | EntType::GoldHarvester
                | EntType::Harvester
                | EntType::Base
                | EntType::Storage => AlphaMode::Mask(0.5),
                _ => AlphaMode::Opaque,
            },
            cull_mode: if let EntType::Storage | EntType::Base = ent_type {
                None
            } else {
                default()
            },
            base_color_texture: match ent_type {
                EntType::Harvester => Some(asset_server.load("crab.png")),
                EntType::Builder => Some(asset_server.load("builder_crab.png")),
                EntType::GoldHarvester => Some(asset_server.load("gold_crab.png")),
                EntType::House => Some(asset_server.load("house.png")),
                EntType::BuilderAcademy => Some(asset_server.load("builder_academy.png")),
                EntType::UpgradeInventory => Some(asset_server.load("gold_academy.png")),
                EntType::Storage => Some(asset_server.load("storage.png")),
                EntType::Base => Some(asset_server.load("base.png")),
                EntType::Road | EntType::Monument => None,
            },
            base_color: match ent_type {
                EntType::Builder | EntType::GoldHarvester | EntType::Harvester => Color::WHITE,
                EntType::House => Color::WHITE,
                EntType::Monument => Color::DARK_GRAY,
                _ => ent_type.color(),
            },
            ..default()
        };
        materials.insert(
            (ent_type, EntState::Hovered),
            material_assets.add({
                let mut material = material.clone();
                material.base_color.set_l(0.2);
                material
            }),
        );
        materials.insert(
            (ent_type, EntState::Placeholder),
            material_assets.add({
                let mut material = material.clone();
                material.unlit = true;
                material.alpha_mode = AlphaMode::Blend;
                material.base_color.set_a(0.5);
                material
            }),
        );
        materials.insert(
            (ent_type, EntState::Preview),
            material_assets.add({
                let mut material = material.clone();
                material.unlit = true;
                material.alpha_mode = AlphaMode::Blend;
                material.base_color.set_a(0.5);
                material
            }),
        );
        materials.insert(
            (ent_type, EntState::BlockedPreview),
            material_assets.add({
                let mut material = material.clone();
                material.base_color_texture = None;
                material.alpha_mode = AlphaMode::Blend;
                material.base_color = Color::rgba(1.0, 0.0, 0.0, 0.5);
                material
            }),
        );
        materials.insert((ent_type, EntState::Normal), material_assets.add(material));
    }

    let ent_materials = EntMaterials {
        meshes,
        materials,
        harvestable_mesh: mesh_assets.add(meshes::make_resource()),
        harvestable_material: material_assets.add(StandardMaterial {
            alpha_mode: AlphaMode::Mask(0.5),
            cull_mode: None,
            base_color_texture: Some(asset_server.load("resource.png")),
            ..default()
        }),
        inventory_thing_material: (0..10)
            .map(|i| {
                material_assets.add(StandardMaterial {
                    base_color: Color::GREEN.with_l(0.2 + i as f32 / 10.0 * 0.5).with_h(
                        thread_rng().gen_range(Color::GREEN.h() - 50.0..Color::GREEN.h() + 50.0),
                    ),
                    fog_enabled: true,
                    ..default()
                })
            })
            .collect(),
        inventory_thing_mesh: mesh_assets.add(Plane::from_size(0.25).into()),
        bavy_mesh: mesh_assets
            .add(Plane::from_size(EntType::Monument.size().max_element() as f32).into()),
        bavy_materials: (0..3)
            .map(|i| {
                material_assets.add(StandardMaterial {
                    alpha_mode: AlphaMode::Mask(0.5),
                    base_color: {
                        let x = 0.5 + i as f32 / 2.0 * 0.5;
                        Color::rgb(x, x, x)
                    },
                    base_color_texture: Some(asset_server.load("bavy.png")),
                    ..default()
                })
            })
            .collect(),
        level_mesh: mesh_assets.add(Plane::from_size(1.0).into()),
        level_material: material_assets.add(StandardMaterial {
            base_color_texture: Some(asset_server.load("level.png")),
            ..default()
        }),
        scaffold_mesh: mesh_assets.add(meshes::scaffold_mesh()),
        scaffold_material: material_assets.add(StandardMaterial {
            alpha_mode: AlphaMode::Mask(0.5),
            cull_mode: None,
            base_color_texture: Some(asset_server.load("scaffolding.png")),
            ..default()
        }),
    };
    commands.insert_resource(ent_materials);
}

fn setup_camera(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn({
        let camera = Camera3dBundle {
            transform: Transform::from_xyz(2.0, 80.0, -20.0)
                .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
            projection: Projection::Perspective(PerspectiveProjection {
                fov: 45.0_f32.to_radians(),
                near: 50.0,
                far: 150.0,
                ..default()
            }),
            tonemapping: Tonemapping::None,
            ..default()
        };
        (
            camera,
            FogSettings {
                color: Color::SEA_GREEN,
                falloff: FogFalloff::Linear {
                    start: 90.0,
                    end: 150.0,
                },
                ..default()
            },
            UiCameraConfig { show_ui: true },
        )
    });
    commands.insert_resource(AmbientLight {
        brightness: 0.7,
        ..default()
    });
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            // TODO: checkbox? shadows_enabled: true,
            color: Color::WHITE,
            illuminance: 3000.0,
            ..default()
        },
        transform: Transform::from_xyz(-3.0, 50.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
    let ground_texture: Handle<Image> = asset_server.load_with_settings("ground.png", {
        let sampler_desc = ImageSamplerDescriptor {
            address_mode_u: ImageAddressMode::Repeat,
            address_mode_v: ImageAddressMode::Repeat,
            ..Default::default()
        };

        move |s: &mut ImageLoaderSettings| {
            s.sampler = ImageSampler::Descriptor(sampler_desc.clone());
        }
    });
    commands.spawn(PbrBundle {
        mesh: meshes.add(meshes::make_plane(1000.0)),
        material: materials.add(StandardMaterial {
            base_color_texture: Some(ground_texture),
            perceptual_roughness: 1.0,
            ..default()
        }),
        transform: Transform::from_xyz(0.0, -0.001, 0.0),
        ..default()
    });
}
