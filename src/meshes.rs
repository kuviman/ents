use bevy::prelude::*;
use bevy::render::mesh::shape::Box;
use bevy::render::{
    mesh::{Indices, Mesh},
    render_resource::PrimitiveTopology,
};

pub fn building_mesh(size: IVec2, floor_height: f32, floors: usize) -> Mesh {
    let size = size.as_vec2();
    let sp = Box {
        min_x: -size.x / 2.0,
        min_z: -size.y / 2.0,
        max_x: size.x / 2.0,
        max_z: size.y / 2.0,
        min_y: floor_height * (1.0 - floors as f32),
        max_y: floor_height,
    };

    let mid_x_uv = size.x / (size.x + size.y);
    let mid_y_uv = (floors as f32 * floor_height) / (floors as f32 * floor_height + size.y);

    let vertices = &[
        // Front
        ([sp.min_x, sp.min_y, sp.max_z], [0.0, 0.0, 1.0], [0.0, 0.0]),
        (
            [sp.max_x, sp.min_y, sp.max_z],
            [0.0, 0.0, 1.0],
            [mid_x_uv, 0.0],
        ),
        (
            [sp.max_x, sp.max_y, sp.max_z],
            [0.0, 0.0, 1.0],
            [mid_x_uv, mid_y_uv],
        ),
        (
            [sp.min_x, sp.max_y, sp.max_z],
            [0.0, 0.0, 1.0],
            [0.0, mid_y_uv],
        ),
        // Back
        (
            [sp.min_x, sp.max_y, sp.min_z],
            [0.0, 0.0, -1.0],
            [mid_x_uv, 0.0],
        ),
        ([sp.max_x, sp.max_y, sp.min_z], [0.0, 0.0, -1.0], [0.0, 0.0]),
        (
            [sp.max_x, sp.min_y, sp.min_z],
            [0.0, 0.0, -1.0],
            [0.0, mid_y_uv],
        ),
        (
            [sp.min_x, sp.min_y, sp.min_z],
            [0.0, 0.0, -1.0],
            [mid_x_uv, mid_y_uv],
        ),
        // Right
        (
            [sp.max_x, sp.min_y, sp.min_z],
            [1.0, 0.0, 0.0],
            [mid_x_uv, 0.0],
        ),
        (
            [sp.max_x, sp.max_y, sp.min_z],
            [1.0, 0.0, 0.0],
            [mid_x_uv, mid_y_uv],
        ),
        (
            [sp.max_x, sp.max_y, sp.max_z],
            [1.0, 0.0, 0.0],
            [1.0, mid_y_uv],
        ),
        ([sp.max_x, sp.min_y, sp.max_z], [1.0, 0.0, 0.0], [1.0, 0.0]),
        // Left
        ([sp.min_x, sp.min_y, sp.max_z], [-1.0, 0.0, 0.0], [1.0, 0.0]),
        (
            [sp.min_x, sp.max_y, sp.max_z],
            [-1.0, 0.0, 0.0],
            [1.0, mid_y_uv],
        ),
        (
            [sp.min_x, sp.max_y, sp.min_z],
            [-1.0, 0.0, 0.0],
            [mid_x_uv, mid_y_uv],
        ),
        (
            [sp.min_x, sp.min_y, sp.min_z],
            [-1.0, 0.0, 0.0],
            [mid_x_uv, 0.0],
        ),
        // Top
        (
            [sp.max_x, sp.max_y, sp.min_z],
            [0.0, 1.0, 0.0],
            [mid_x_uv, mid_y_uv],
        ),
        (
            [sp.min_x, sp.max_y, sp.min_z],
            [0.0, 1.0, 0.0],
            [0.0, mid_y_uv],
        ),
        ([sp.min_x, sp.max_y, sp.max_z], [0.0, 1.0, 0.0], [0.0, 1.0]),
        (
            [sp.max_x, sp.max_y, sp.max_z],
            [0.0, 1.0, 0.0],
            [mid_x_uv, 1.0],
        ),
    ];

    let positions: Vec<_> = vertices.iter().map(|(p, _, _)| *p).collect();
    let normals: Vec<_> = vertices.iter().map(|(_, n, _)| *n).collect();
    let uvs: Vec<_> = vertices
        .iter()
        .map(|(_, _, uv)| [uv[0], 1.0 - uv[1]])
        .collect();

    let indices = Indices::U32(vec![
        0, 1, 2, 2, 3, 0, // front
        4, 5, 6, 6, 7, 4, // back
        8, 9, 10, 10, 11, 8, // right
        12, 13, 14, 14, 15, 12, // left
        16, 17, 18, 18, 19, 16, // top
    ]);

    Mesh::new(PrimitiveTopology::TriangleList)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_indices(Some(indices))
}
