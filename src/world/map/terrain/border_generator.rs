use std::cmp::PartialEq;
use image::GrayImage;
use crate::world::map::terrain::terrain_generator::{HEIGHT_SCALE, VOID_HEIGHT};

#[derive(Debug, Clone, Copy, PartialEq)]
enum Direction {
    Left,
    Right,
    Top,
    Bottom
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum BorderType {
    Corner,
    Line,
    DeadEnd,
    Canyon,
    Pit,
    None
}

struct RelativeCoord {
    forward: i32,
    right: i32,
}

pub fn make_borders(
    start_x: u32,
    start_z: u32,
    chunk_size: u32,
    heightmap: &GrayImage,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    indices: &mut Vec<u32>
) {
    let end_x = (start_x + chunk_size).min(heightmap.width());
    let end_z = (start_z + chunk_size).min(heightmap.height());

    for z in start_z..end_z {
        for x in start_x..end_x {
            if heightmap.get_pixel(x, z)[0] == VOID_HEIGHT {
                continue;
            }

            if is_border_pixel(x, z, heightmap) {
                if x > 0 && heightmap.get_pixel(x-1, z)[0] == VOID_HEIGHT {
                    process_border(x-1, z, Direction::Left, heightmap, positions, normals, uvs, indices);
                }
                if x+1 < heightmap.width() && heightmap.get_pixel(x+1, z)[0] == VOID_HEIGHT {
                    process_border(x+1, z, Direction::Right, heightmap, positions, normals, uvs, indices);
                }
                if z > 0 && heightmap.get_pixel(x, z-1)[0] == VOID_HEIGHT {
                    process_border(x, z-1, Direction::Top, heightmap, positions, normals, uvs, indices);
                }
                if z+1 < heightmap.height() && heightmap.get_pixel(x, z+1)[0] == VOID_HEIGHT {
                    process_border(x, z+1, Direction::Bottom, heightmap, positions, normals, uvs, indices);
                }
            }
        }
    }
}

fn process_border(
    x: u32,
    z: u32,
    direction: Direction,
    heightmap: &GrayImage,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    indices: &mut Vec<u32>
) {
    let border_type = determine_border_type(x, z, direction, heightmap);

    match border_type {
        BorderType::Line => process_line_border(x, z, direction, positions, normals, uvs, indices, heightmap),
        _ => println!("Border at x: {}, z: {}, direction: {:?}, type: {:?}", x, z, direction, border_type)
    }
}

fn determine_border_type(x: u32, z: u32, direction: Direction, heightmap: &GrayImage) -> BorderType {
    let forward = check_relative_pixel(x, z, direction, RelativeCoord { forward: 1, right: 0 }, heightmap);
    let right = check_relative_pixel(x, z, direction, RelativeCoord { forward: 0, right: 1 }, heightmap);
    let left = check_relative_pixel(x, z, direction, RelativeCoord { forward: 0, right: -1 }, heightmap);

    if !forward && right && !left {
        BorderType::Corner
    } else if !forward && !right && !left {
        BorderType::Line
    } else {
        BorderType::None
    }
}

fn check_relative_pixel(
    base_x: u32,
    base_z: u32,
    direction: Direction,
    rel: RelativeCoord,
    heightmap: &GrayImage
) -> bool {
    let pixel = get_relative_pixel(base_x, base_z, direction, rel, heightmap);

    if pixel.is_none() {
        return false;
    }

    pixel.unwrap() > VOID_HEIGHT
}

fn get_relative_pixel(
    base_x: u32,
    base_z: u32,
    direction: Direction,
    rel: RelativeCoord,
    heightmap: &GrayImage
) -> Option<u8> {
    let (nx, nz) = relative_to_absolute(base_x, base_z, direction, rel);

    if nx >= heightmap.width() || nz >= heightmap.height() || nx < 0 || nz < 0 {
        return None;
    }
    if (base_x == 1503 && base_z == 3433) {
        println!("x {:?}, y {:?}, dir {:?}, height {:?}", nx, nz, direction, heightmap.get_pixel(nx, nz)[0]);
    }

    Some(heightmap.get_pixel(nx, nz)[0])
}

fn relative_to_absolute(
    base_x: u32,
    base_z: u32,
    direction: Direction,
    rel: RelativeCoord
) -> (u32, u32) {
    let (dx, dz) = match direction {
        Direction::Right => (-rel.right, -rel.forward), // 1, 1 -> 1, -1
        Direction::Left => (rel.right, rel.forward), // 1, 1 -> -1, 1
        Direction::Bottom => (-rel.right, rel.forward), // 1, 1 -> 1, 1
        Direction::Top => (rel.right, -rel.forward), // 1, 1 -> 1, 1
    };

    let abs_x = if dx < 0 && base_x < dx.abs() as u32 {
        0
    } else {
        (base_x as i32 + dx) as u32
    };

    let abs_z = if dz < 0 && base_z < dz.abs() as u32 {
        0
    } else {
        (base_z as i32 + dz) as u32
    };

    (abs_x, abs_z)
}

fn is_border_pixel(x: u32, z: u32, heightmap: &GrayImage) -> bool {
    let directions = [(0, -1), (1, 0), (0, 1), (-1, 0)];

    for (dx, dz) in directions {
        let nx = (x as i32 + dx) as u32;
        let nz = (z as i32 + dz) as u32;

        if nx < heightmap.width() && nz < heightmap.height() && heightmap.get_pixel(nx, nz)[0] == VOID_HEIGHT {
            return true;
        }
    }

    false
}

fn process_line_border(
    x: u32,
    z: u32,
    direction: Direction,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    indices: &mut Vec<u32>,
    heightmap: &GrayImage
) {
    let height = heightmap.get_pixel(x, z)[0] as f32 * HEIGHT_SCALE;

    println!("Created Line border at x: {}, z: {}, direction: {:?}", x, z, direction);
}
