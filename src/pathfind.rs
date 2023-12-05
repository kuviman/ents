use std::{collections::BinaryHeap, marker::PhantomData};

use bevy::{prelude::*, utils::HashMap};
use rand::{seq::SliceRandom, thread_rng};

use crate::{
    chunks::GeneratedChunks,
    game::MOVE_DIRECTIONS,
    tile_map::{Pos, TileMap},
};

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, despawn_debug);
    }
}

pub trait AppExt {
    fn register_pathfinding_towards<C: Component>(&mut self);
}

impl AppExt for App {
    fn register_pathfinding_towards<C: Component>(&mut self) {
        self.insert_resource(Pathfinding::<C> {
            closest: default(),
            updates: default(),
            phantom_data: PhantomData,
        });
        self.add_systems(Update, (detect_map_updates::<C>, pathfind_iteration::<C>));
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
struct Closest {
    distance: u32,
    ways: f64,
}

#[derive(Resource)]
pub struct Pathfinding<T> {
    closest: HashMap<IVec2, Closest>,
    updates: BinaryHeap<Update>,
    phantom_data: PhantomData<T>,
}

pub struct Direction {
    pub dir: IVec2,
    pub distance: u32,
}

impl<T> Pathfinding<T> {
    pub fn pathfind(&self, from: IVec2) -> Option<Direction> {
        let closest_distance = MOVE_DIRECTIONS
            .into_iter()
            .filter_map(|dir| self.closest.get(&(from + dir)))
            .map(|closest| closest.distance)
            .min()?;
        let dir = MOVE_DIRECTIONS
            .choose_weighted(&mut thread_rng(), |&dir| {
                self.closest.get(&(from + dir)).map_or(0.0, |closest| {
                    if closest.distance == closest_distance {
                        closest.ways
                    } else {
                        0.0
                    }
                })
            })
            .ok()
            .copied()?;
        Some(Direction {
            dir,
            distance: self.closest[&(from + dir)].distance + 1,
        })
    }
}

#[derive(PartialEq, Eq)]
struct Update {
    distance: u64,
    pos: IVec2,
}

impl Update {
    fn new(pos: IVec2) -> Self {
        Self { distance: 0, pos }
    }
}

impl PartialOrd for Update {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Update {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.distance.cmp(&self.distance)
    }
}

fn detect_map_updates<T: Component>(
    mut data: ResMut<Pathfinding<T>>,
    map_updates: Query<(Entity, &Pos), (Or<(Changed<Pos>, Added<T>)>, With<T>)>,
    mut removed: RemovedComponents<T>,
    mut prev_pos: Local<HashMap<Entity, IVec2>>,
) {
    for (entity, pos) in map_updates.iter() {
        data.updates.push(Update::new(pos.0));
        if let Some(&prev_pos) = prev_pos.get(&entity) {
            data.updates.push(Update::new(prev_pos));
        }
        prev_pos.insert(entity, pos.0);
    }
    for entity in removed.read() {
        if let Some(prev_pos) = prev_pos.remove(&entity) {
            data.updates.push(Update::new(prev_pos));
        }
    }
}

#[derive(Component)]
struct DebugThing(f32);

fn despawn_debug(mut q: Query<(Entity, &mut DebugThing)>, mut commands: Commands, time: Res<Time>) {
    for (entity, mut debug) in q.iter_mut() {
        debug.0 += time.delta_seconds();
        if debug.0 > 1.0 {
            commands.entity(entity).despawn();
        }
    }
}

fn pathfind_iteration<T: Component>(
    entities: Query<(), With<T>>,
    tile_map: Res<TileMap>,
    mut data: ResMut<Pathfinding<T>>,
    generated_chunks: Res<GeneratedChunks>,
    mut commands: Commands,
) {
    let mut iterations_left = 1000; // TODO base on time?
    while let Some(update) = data.updates.pop() {
        let new_closest = if tile_map
            .entities_at(update.pos)
            .any(|entity| entities.contains(entity))
        {
            Some(Closest {
                distance: 0,
                ways: 1.0,
            })
        } else {
            let mut closest = None;
            for dir in MOVE_DIRECTIONS {
                let next_pos = update.pos + dir;
                if let Some(next_closest) = data.closest.get(&next_pos) {
                    let do_replace = match &mut closest {
                        Some(Closest { distance, ways }) => {
                            if *distance == next_closest.distance + 1 {
                                *ways += next_closest.ways;
                                false
                            } else {
                                *distance > next_closest.distance + 1
                            }
                        }
                        None => true,
                    };
                    if do_replace {
                        closest = Some(Closest {
                            distance: next_closest.distance + 1,
                            ways: next_closest.ways,
                        });
                    };
                }
            }
            closest
        };

        let old = data.closest.get(&update.pos);
        if old != new_closest.as_ref() {
            commands.spawn((
                Text2dBundle {
                    text: Text::from_section(
                        new_closest
                            .as_ref()
                            .map_or("none".to_owned(), |closest| closest.distance.to_string()),
                        default(),
                    ),
                    transform: Transform::from_translation(update.pos.as_vec2().extend(1.0))
                        .with_scale(Vec3::splat(0.1)),
                    ..default()
                },
                DebugThing(0.0),
            ));
            match new_closest {
                Some(new) => {
                    data.closest.insert(update.pos, new);
                }
                None => {
                    data.closest.remove(&update.pos);
                }
            }
            for dir in MOVE_DIRECTIONS {
                let next_pos = update.pos + dir;
                if generated_chunks.is_generated(next_pos) {
                    data.updates.push(Update {
                        distance: update.distance + 1,
                        pos: next_pos,
                    });
                }
            }
        }

        iterations_left -= 1;
        if iterations_left == 0 {
            break;
        }
    }
}