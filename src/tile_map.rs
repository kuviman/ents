use bevy::{
    prelude::*,
    utils::{HashMap, HashSet},
};

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TileMap {
            entities_by_tile: default(),
            prev: default(),
        });
        app.add_systems(PreUpdate, update_tile_map);
    }
}

#[derive(Component)]
pub struct GridCoords(pub IVec2);

#[derive(Resource)]
pub struct TileMap {
    entities_by_tile: HashMap<IVec2, HashSet<Entity>>,
    prev: HashMap<Entity, IVec2>,
}

impl TileMap {
    pub fn entities_at(&self, pos: IVec2) -> impl Iterator<Item = Entity> + '_ {
        self.entities_by_tile
            .get(&pos)
            .into_iter()
            .flatten()
            .copied()
    }
}

fn update_tile_map(
    q: Query<(Entity, &GridCoords), Changed<GridCoords>>,
    mut tile_map: ResMut<TileMap>,
    mut removed: RemovedComponents<GridCoords>,
) {
    let tile_map = &mut *tile_map;
    for entity in removed.read() {
        if let Some(prev) = tile_map.prev.get(&entity) {
            tile_map
                .entities_by_tile
                .get_mut(prev)
                .unwrap()
                .remove(&entity);
        }
    }
    for (entity, coords) in q.iter() {
        if let Some(prev) = tile_map.prev.get(&entity) {
            tile_map
                .entities_by_tile
                .get_mut(prev)
                .unwrap()
                .remove(&entity);
        }
        tile_map
            .entities_by_tile
            .entry(coords.0)
            .or_default()
            .insert(entity);
        tile_map.prev.insert(entity, coords.0);
    }
}
