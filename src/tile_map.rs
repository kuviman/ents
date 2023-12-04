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
pub struct Pos(pub IVec2);

#[derive(Component)]
pub struct Size(pub IVec2);

#[derive(Resource)]
pub struct TileMap {
    entities_by_tile: HashMap<IVec2, HashSet<Entity>>,
    prev: HashMap<Entity, (IVec2, IVec2)>,
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
    q: Query<(Entity, &Pos, Option<&Size>), Changed<Pos>>,
    mut tile_map: ResMut<TileMap>,
    mut removed: RemovedComponents<Pos>,
) {
    let tile_map = &mut *tile_map;
    for entity in removed.read() {
        if let Some((prev_pos, prev_size)) = tile_map.prev.remove(&entity) {
            for dx in 0..prev_size.x {
                for dy in 0..prev_size.y {
                    tile_map
                        .entities_by_tile
                        .get_mut(&(prev_pos + IVec2::new(dx, dy)))
                        .unwrap()
                        .remove(&entity);
                }
            }
        }
    }
    for (entity, coords, size) in q.iter() {
        let size = size.map_or(IVec2::splat(1), |size| size.0);
        if let Some(&(prev_pos, prev_size)) = tile_map.prev.get(&entity) {
            for dx in 0..prev_size.x {
                for dy in 0..prev_size.y {
                    tile_map
                        .entities_by_tile
                        .get_mut(&(prev_pos + IVec2::new(dx, dy)))
                        .unwrap()
                        .remove(&entity);
                }
            }
        }
        for dx in 0..size.x {
            for dy in 0..size.y {
                tile_map
                    .entities_by_tile
                    .entry(coords.0 + IVec2::new(dx, dy))
                    .or_default()
                    .insert(entity);
            }
        }
        tile_map.prev.insert(entity, (coords.0, size));
    }
}
