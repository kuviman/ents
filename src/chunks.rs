use bevy::{prelude::*, utils::HashSet};

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GeneratedChunks(default()));
        app.add_event::<GenerateChunk>();
        app.add_systems(Update, calculate_chunks_to_generate);
    }
}

fn calculate_chunks_to_generate(
    camera: Query<(&GlobalTransform, &Camera)>,
    mut generated_chunks: ResMut<GeneratedChunks>,
    mut event_writer: EventWriter<GenerateChunk>,
) {
    let (camera_transform, camera) = camera.single();
    let Some(window_viewport) = camera.logical_viewport_rect() else {
        return;
    };
    let viewport = [
        window_viewport.min,
        Vec2::new(window_viewport.min.x, window_viewport.max.y),
        window_viewport.max,
        Vec2::new(window_viewport.max.x, window_viewport.min.y),
    ]
    .into_iter()
    .filter_map(|p| camera.viewport_to_world(camera_transform, p))
    .map(|ray| {
        let t = -ray.origin.y / ray.direction.y;
        (ray.origin + ray.direction * t).xz()
    })
    .map(|p| Rect::from_corners(p, p))
    .reduce(|a, b| Rect::union(&a, b))
    .unwrap();

    for chunk_x in (viewport.min.x / CHUNK_SIZE as f32).floor() as i32
        ..(viewport.max.x / CHUNK_SIZE as f32).ceil() as i32
    {
        for chunk_y in (viewport.min.y / CHUNK_SIZE as f32).floor() as i32
            ..(viewport.max.y / CHUNK_SIZE as f32).ceil() as i32
        {
            let chunk_pos = IVec2::new(chunk_x, chunk_y);
            if generated_chunks.0.contains(&chunk_pos) {
                continue;
            }
            generated_chunks.0.insert(chunk_pos);
            event_writer.send(GenerateChunk(chunk_pos));
        }
    }
}

const CHUNK_SIZE: i32 = 64;

#[derive(Resource)]
pub struct GeneratedChunks(HashSet<IVec2>);

impl GeneratedChunks {
    pub fn is_generated(&self, pos: IVec2) -> bool {
        let chunk_pos = pos.div_euclid(IVec2::splat(CHUNK_SIZE));
        self.0.contains(&chunk_pos)
    }
}

#[derive(Event)]
pub struct GenerateChunk(IVec2);

impl GenerateChunk {
    pub fn rect(&self) -> IRect {
        IRect::from_corners(self.0 * CHUNK_SIZE, (self.0 + IVec2::splat(1)) * CHUNK_SIZE)
    }
}
