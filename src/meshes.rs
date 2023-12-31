use bevy::prelude::*;
use bevy::render::mesh::shape::Box;
use bevy::render::{
    mesh::{Indices, Mesh},
    render_resource::PrimitiveTopology,
};

pub fn make_plane(size: f32) -> Mesh {
    // here this is split in the z and x directions if one ever needs asymmetrical subdivision
    // two Plane struct fields would need to be added instead of the single subdivisions field
    let z_vertex_count = 2;
    let x_vertex_count = 2;
    let num_vertices = (z_vertex_count * x_vertex_count) as usize;
    let num_indices = ((z_vertex_count - 1) * (x_vertex_count - 1) * 6) as usize;
    let up = Vec3::Y.to_array();

    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(num_vertices);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(num_vertices);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(num_vertices);
    let mut indices: Vec<u32> = Vec::with_capacity(num_indices);

    for z in 0..z_vertex_count {
        for x in 0..x_vertex_count {
            let tx = x as f32 / (x_vertex_count - 1) as f32;
            let tz = z as f32 / (z_vertex_count - 1) as f32;
            let pos = [(-0.5 + tx) * size, 0.0, (-0.5 + tz) * size];
            positions.push(pos);
            normals.push(up);
            uvs.push([pos[0], pos[2]]);
        }
    }

    for y in 0..z_vertex_count - 1 {
        for x in 0..x_vertex_count - 1 {
            let quad = y * x_vertex_count + x;
            indices.push(quad + x_vertex_count + 1);
            indices.push(quad + 1);
            indices.push(quad + x_vertex_count);
            indices.push(quad);
            indices.push(quad + x_vertex_count);
            indices.push(quad + 1);
        }
    }

    Mesh::new(PrimitiveTopology::TriangleList)
        .with_indices(Some(Indices::U32(indices)))
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
}

pub fn make_resource() -> Mesh {
    let height = 20;
    let mut positions = Vec::<Vec3>::new();
    let mut normals = Vec::<Vec3>::new();
    let mut uvs = Vec::<Vec2>::new();
    let mut indices = Vec::<usize>::new();
    let mut add_quad = |quad: [(Vec3, Vec3, Vec2); 4]| {
        let s = positions.len();
        for (pos, normal, uv) in quad {
            positions.push(pos);
            normals.push(normal);
            uvs.push(uv);
        }
        indices.push(s);
        indices.push(s + 1);
        indices.push(s + 2);
        indices.push(s);
        indices.push(s + 2);
        indices.push(s + 3);
    };
    let mid_uv = 1.0 / (1.0 + height as f32);
    for i in 0..height {
        let h = 0.5 - i as f32;
        add_quad([
            (Vec3::new(-0.5, h, -0.5), Vec3::Y, Vec2::new(0.0, 0.0)),
            (Vec3::new(0.5, h, -0.5), Vec3::Y, Vec2::new(1.0, 0.0)),
            (Vec3::new(0.5, h, 0.5), Vec3::Y, Vec2::new(1.0, mid_uv)),
            (Vec3::new(-0.5, h, 0.5), Vec3::Y, Vec2::new(0.0, mid_uv)),
        ]);
    }

    let bottom = -height as f32 + 1.0;
    let top = 1.0;
    add_quad([
        (
            Vec3::new(-0.5, bottom, -0.5),
            Vec3::new(-1.0, 0.0, 1.0).normalize(),
            Vec2::new(0.0, 1.0),
        ),
        (
            Vec3::new(0.5, bottom, 0.5),
            Vec3::new(-1.0, 0.0, 1.0).normalize(),
            Vec2::new(1.0, 1.0),
        ),
        (
            Vec3::new(0.5, top, 0.5),
            Vec3::new(-1.0, 0.0, 1.0).normalize(),
            Vec2::new(1.0, mid_uv),
        ),
        (
            Vec3::new(-0.5, top, -0.5),
            Vec3::new(-1.0, 0.0, 1.0).normalize(),
            Vec2::new(0.0, mid_uv),
        ),
    ]);
    add_quad([
        (
            Vec3::new(0.5, bottom, -0.5),
            Vec3::new(-1.0, 0.0, 1.0).normalize(),
            Vec2::new(0.0, 1.0),
        ),
        (
            Vec3::new(-0.5, bottom, 0.5),
            Vec3::new(-1.0, 0.0, 1.0).normalize(),
            Vec2::new(1.0, 1.0),
        ),
        (
            Vec3::new(-0.5, top, 0.5),
            Vec3::new(-1.0, 0.0, 1.0).normalize(),
            Vec2::new(1.0, mid_uv),
        ),
        (
            Vec3::new(0.5, top, -0.5),
            Vec3::new(-1.0, 0.0, 1.0).normalize(),
            Vec2::new(0.0, mid_uv),
        ),
    ]);

    Mesh::new(PrimitiveTopology::TriangleList)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_indices(Some(Indices::U32(
            indices.into_iter().map(|x| x as u32).collect(),
        )))
}

pub fn scaffold_mesh() -> Mesh {
    let sp = Box {
        min_x: -0.5,
        min_z: -0.5,
        max_x: 0.5,
        max_z: 0.5,
        min_y: -9.0,
        max_y: 0.3,
    };

    let vertices = &[
        // Front
        ([sp.min_x, sp.min_y, sp.max_z], [0.0, 0.0, 1.0], [0.0, 0.0]),
        ([sp.max_x, sp.min_y, sp.max_z], [0.0, 0.0, 1.0], [1.0, 0.0]),
        ([sp.max_x, sp.max_y, sp.max_z], [0.0, 0.0, 1.0], [1.0, 1.0]),
        ([sp.min_x, sp.max_y, sp.max_z], [0.0, 0.0, 1.0], [0.0, 1.0]),
        // Back
        ([sp.min_x, sp.max_y, sp.min_z], [0.0, 0.0, -1.0], [0.0, 1.0]),
        ([sp.max_x, sp.max_y, sp.min_z], [0.0, 0.0, -1.0], [1.0, 1.0]),
        ([sp.max_x, sp.min_y, sp.min_z], [0.0, 0.0, -1.0], [1.0, 0.0]),
        ([sp.min_x, sp.min_y, sp.min_z], [0.0, 0.0, -1.0], [0.0, 0.0]),
        // Right
        ([sp.max_x, sp.min_y, sp.min_z], [1.0, 0.0, 0.0], [0.0, 0.0]),
        ([sp.max_x, sp.max_y, sp.min_z], [1.0, 0.0, 0.0], [0.0, 1.0]),
        ([sp.max_x, sp.max_y, sp.max_z], [1.0, 0.0, 0.0], [1.0, 1.0]),
        ([sp.max_x, sp.min_y, sp.max_z], [1.0, 0.0, 0.0], [1.0, 0.0]),
        // Left
        ([sp.min_x, sp.min_y, sp.max_z], [-1.0, 0.0, 0.0], [1.0, 0.0]),
        ([sp.min_x, sp.max_y, sp.max_z], [-1.0, 0.0, 0.0], [1.0, 1.0]),
        ([sp.min_x, sp.max_y, sp.min_z], [-1.0, 0.0, 0.0], [0.0, 1.0]),
        ([sp.min_x, sp.min_y, sp.min_z], [-1.0, 0.0, 0.0], [0.0, 0.0]),
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
    ]);

    Mesh::new(PrimitiveTopology::TriangleList)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_indices(Some(indices))
}

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
            [0.0, mid_y_uv],
        ),
        (
            [sp.max_x, sp.max_y, sp.min_z],
            [0.0, 0.0, -1.0],
            [mid_x_uv, mid_y_uv],
        ),
        (
            [sp.max_x, sp.min_y, sp.min_z],
            [0.0, 0.0, -1.0],
            [mid_x_uv, 0.0],
        ),
        ([sp.min_x, sp.min_y, sp.min_z], [0.0, 0.0, -1.0], [0.0, 0.0]),
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
