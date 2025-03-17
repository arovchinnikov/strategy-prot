use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use image::GrayImage;

pub fn generate_terrain_mesh(
    start_x: u32,
    start_z: u32,
    heightmap: &GrayImage
) -> Option<Mesh> {
    let size = 128;
    let max_height = 100.0;
    let overlap = 0;

    // Проверяем, достаточно ли большая карта высот, учитывая перекрытие
    if start_x + size + overlap >= heightmap.width() || start_z + size + overlap >= heightmap.height() {
        return None;
    }

    let mut vertices: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    // Проходим по всем точкам в пределах чанка, включая перекрытие
    let points_width = size + overlap + 1;
    let points_height = size + overlap + 1;
    let mut has_non_zero_height = false;

    // Сначала собираем все высоты
    let mut heights = vec![vec![0.0; points_width as usize]; points_height as usize];

    for z in 0..points_height {
        for x in 0..points_width {
            let heightmap_x = start_x + x;
            let heightmap_z = start_z + z;

            // Получаем высоту из карты высот
            let pixel = heightmap.get_pixel(heightmap_x, heightmap_z);
            let height = if pixel[0] > 0 {
                let h = (pixel[0] as f32 / 255.0) * max_height;
                if h > 0.0 {
                    has_non_zero_height = true;
                }
                h
            } else {
                0.0 // Если пиксель черный, высота равна 0
            };

            heights[z as usize][x as usize] = height;
        }
    }

    // Если во всем чанке нет точек с ненулевой высотой, возвращаем None
    if !has_non_zero_height {
        return None;
    }

    // Создаем вершины и их атрибуты
    for z in 0..points_height {
        for x in 0..points_width {
            let height = heights[z as usize][x as usize];
            vertices.push([x as f32, height / 2.0, z as f32]);
            uvs.push([x as f32 / (size + overlap) as f32, z as f32 / (size + overlap) as f32]);
            normals.push([0.0, 1.0, 0.0]); // Временные нормали
        }
    }

    // Создаем треугольники (индексы)
    for z in 0..size + overlap {
        for x in 0..size + overlap {
            let top_left = z * points_width + x;
            let top_right = top_left + 1;
            let bottom_left = (z + 1) * points_width + x;
            let bottom_right = bottom_left + 1;

            let h_tl = heights[z as usize][x as usize];
            let h_tr = heights[z as usize][(x + 1) as usize];
            let h_bl = heights[(z + 1) as usize][x as usize];
            let h_br = heights[(z + 1) as usize][(x + 1) as usize];

            // Пропускаем квадрат, если хотя бы одна из его вершин имеет нулевую высоту
            if h_tl == 0.0 || h_tr == 0.0 || h_bl == 0.0 || h_br == 0.0 {
                continue;
            }

            // Первый треугольник (против часовой стрелки)
            indices.push(top_left);
            indices.push(bottom_left);
            indices.push(top_right);

            // Второй треугольник (против часовой стрелки)
            indices.push(top_right);
            indices.push(bottom_left);
            indices.push(bottom_right);
        }
    }

    if indices.is_empty() {
        return None;
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));

    // Вычисляем нормали для правильного освещения
    mesh.compute_smooth_normals();

    Some(mesh)
}
