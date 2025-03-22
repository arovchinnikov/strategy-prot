use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use image::GrayImage;
use std::collections::HashMap;

const VOID_HEIGHT: u8 = 0;
const HEIGHT_SCALE: f32 = 0.3;

struct ChunkBounds {
    start_x: u32,
    start_z: u32,
    expanded_start_x: u32,
    expanded_start_z: u32,
    expanded_end_x: u32,
    expanded_end_z: u32,
    expanded_width: usize,
    expanded_depth: usize,
}

pub fn generate_terrain_mesh(
    start_x: u32,
    start_z: u32,
    heightmap: &GrayImage,
) -> Option<Mesh> {
    let chunk_size = 128;

    let bounds = calculate_chunk_bounds(start_x, start_z, chunk_size, heightmap);
    if bounds.expanded_end_x <= bounds.expanded_start_x || bounds.expanded_end_z <= bounds.expanded_start_z {
        return None;
    }

    if is_chunk_empty(start_x, start_z, chunk_size, heightmap) {
        return None;
    }

    let (
        positions,
        normals,
        uvs,
        indices,
        vertex_map
    ) = generate_mesh_data(bounds, heightmap);

    if indices.is_empty() {
        return None;
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));

    Some(mesh)
}

fn calculate_chunk_bounds(start_x: u32, start_z: u32, chunk_size: u32, heightmap: &GrayImage) -> ChunkBounds {
    let expanded_start_x = start_x.saturating_sub(1);
    let expanded_start_z = start_z.saturating_sub(1);
    let expanded_end_x = (start_x + chunk_size + 1).min(heightmap.width());
    let expanded_end_z = (start_z + chunk_size + 1).min(heightmap.height());

    ChunkBounds {
        start_x,
        start_z,
        expanded_start_x,
        expanded_start_z,
        expanded_end_x,
        expanded_end_z,
        expanded_width: (expanded_end_x - expanded_start_x) as usize,
        expanded_depth: (expanded_end_z - expanded_start_z) as usize,
    }
}

fn is_chunk_empty(start_x: u32, start_z: u32, chunk_size: u32, heightmap: &GrayImage) -> bool {
    let end_x = start_x + chunk_size.min(heightmap.width() - start_x);
    let end_z = start_z + chunk_size.min(heightmap.height() - start_z);

    for z in start_z..end_z {
        for x in start_x..end_x {
            if heightmap.get_pixel(x, z)[0] != VOID_HEIGHT {
                return false;
            }
        }
    }

    true
}

fn generate_mesh_data(
    bounds: ChunkBounds,
    heightmap: &GrayImage
) -> (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<[f32; 2]>, Vec<u32>, HashMap<(u32, u32), u32>) {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut vertex_map: HashMap<(u32, u32), u32> = HashMap::new();

    create_vertices(&bounds, heightmap, &mut positions, &mut normals, &mut uvs, &mut vertex_map);
    let all_indices = create_triangles(&bounds, heightmap, &vertex_map, &mut indices);
    calculate_normals(&all_indices, &positions, &mut normals);

    (positions, normals, uvs, indices, vertex_map)
}

fn create_vertices(
    bounds: &ChunkBounds,
    heightmap: &GrayImage,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    vertex_map: &mut HashMap<(u32, u32), u32>
) {
    for z in bounds.expanded_start_z..bounds.expanded_end_z + 1 {
        for x in bounds.expanded_start_x..bounds.expanded_end_x + 1 {
            if !is_vertex_needed(x, z, heightmap) {
                continue;
            }

            let height = calculate_vertex_height(x, z, heightmap);
            let local_x = (x as i32 - bounds.start_x as i32) as f32;
            let local_z = (z as i32 - bounds.start_z as i32) as f32;
            let vertex_index = positions.len() as u32;
            positions.push([local_x, height, local_z]);
            normals.push([0.0, 1.0, 0.0]);
            uvs.push([
                (x - bounds.expanded_start_x) as f32 / bounds.expanded_width as f32,
                (z - bounds.expanded_start_z) as f32 / bounds.expanded_depth as f32
            ]);

            vertex_map.insert((x, z), vertex_index);
        }
    }
}

fn is_vertex_needed(
    x: u32, z: u32,
    heightmap: &GrayImage
) -> bool {
    check_pixel(x.saturating_sub(1), z.saturating_sub(1), heightmap)
        || check_pixel(x, z.saturating_sub(1), heightmap)
        || check_pixel(x.saturating_sub(1), z, heightmap)
        || check_pixel(x, z, heightmap)
}

fn check_pixel(x: u32, z: u32, heightmap: &GrayImage) -> bool {
    x < heightmap.width() && z < heightmap.height() &&
        heightmap.get_pixel(x, z)[0] != VOID_HEIGHT
}

fn calculate_vertex_height(
    x: u32, z: u32,
    heightmap: &GrayImage
) -> f32 {
    let mut height_sum = 0.0;
    let mut count = 0;

    let adjacent_pixels = [
        (x.saturating_sub(1), z.saturating_sub(1)),
        (x, z.saturating_sub(1)),
        (x.saturating_sub(1), z),
        (x, z)
    ];

    for (px, pz) in adjacent_pixels {
        if check_pixel(px, pz, heightmap) {
            height_sum += heightmap.get_pixel(px, pz)[0] as f32;
            count += 1;
        }
    }

    if count > 0 {
        (height_sum / count as f32) * HEIGHT_SCALE
    } else {
        0.0
    }
}

fn create_triangles(
    bounds: &ChunkBounds,
    heightmap: &GrayImage,
    vertex_map: &HashMap<(u32, u32), u32>,
    indices: &mut Vec<u32>
) -> Vec<u32> {
    let mut all_indices: Vec<u32> = Vec::new();

    for z in bounds.expanded_start_z..bounds.expanded_end_z {
        for x in bounds.expanded_start_x..bounds.expanded_end_x {
            if x >= heightmap.width() || z >= heightmap.height() ||
                heightmap.get_pixel(x, z)[0] == VOID_HEIGHT {
                continue;
            }

            if let (
                Some(&top_left),
                Some(&top_right),
                Some(&bottom_right),
                Some(&bottom_left)
            ) = (
                vertex_map.get(&(x, z)),
                vertex_map.get(&(x+1, z)),
                vertex_map.get(&(x+1, z+1)),
                vertex_map.get(&(x, z+1))
            ) {
                all_indices.push(top_right);
                all_indices.push(top_left);
                all_indices.push(bottom_right);

                all_indices.push(top_left);
                all_indices.push(bottom_left);
                all_indices.push(bottom_right);

                if is_in_original_chunk(x, z, bounds.start_x, bounds.start_z) {
                    indices.push(top_right);
                    indices.push(top_left);
                    indices.push(bottom_right);

                    indices.push(top_left);
                    indices.push(bottom_left);
                    indices.push(bottom_right);
                }
            }
        }
    }

    all_indices
}

fn is_in_original_chunk(x: u32, z: u32, start_x: u32, start_z: u32) -> bool {
    let chunk_size = 128;
    x >= start_x && x < start_x + chunk_size && z >= start_z && z < start_z + chunk_size
}

fn calculate_normals(
    all_indices: &[u32],
    positions: &[[f32; 3]],
    normals: &mut [[f32; 3]]
) {
    for normal in normals.iter_mut() {
        *normal = [0.0, 0.0, 0.0];
    }

    for i in (0..all_indices.len()).step_by(3) {
        if i + 2 >= all_indices.len() {
            continue;
        }

        let i0 = all_indices[i] as usize;
        let i1 = all_indices[i + 1] as usize;
        let i2 = all_indices[i + 2] as usize;
        let p0 = positions[i0];
        let p1 = positions[i1];
        let p2 = positions[i2];
        let v1 = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
        let v2 = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];
        let normal = [
            v1[1] * v2[2] - v1[2] * v2[1],
            v1[2] * v2[0] - v1[0] * v2[2],
            v1[0] * v2[1] - v1[1] * v2[0]
        ];
        for &idx in &[i0, i1, i2] {
            normals[idx][0] += normal[0];
            normals[idx][1] += normal[1];
            normals[idx][2] += normal[2];
        }
    }

    for normal in normals.iter_mut() {
        let length = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();

        if length > 0.0001 {
            normal[0] /= length;
            normal[1] /= length;
            normal[2] /= length;
        } else {
            *normal = [0.0, 1.0, 0.0];
        }
    }
}
