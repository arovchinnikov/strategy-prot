use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use image::GrayImage;

pub fn generate_terrain_mesh(
    start_x: u32,
    start_z: u32,
    heightmap: &GrayImage,
    overlap: u32, // Добавляем параметр перекрытия для сшивания чанков
) -> Option<Mesh> {
    let size = 128;
    let max_height = 100.0;

    // Проверяем, достаточно ли большая карта высот, учитывая перекрытие
    if start_x + size + overlap > heightmap.width() || start_z + size + overlap > heightmap.height() {
        return None;
    }

    let mut vertices: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    let mut has_valid_height = false;

    // Создаем двумерный массив для хранения индексов вершин, или None если вершины нет
    let mut vertex_indices = vec![vec![None::<u32>; (size + overlap) as usize]; (size + overlap) as usize];

    // Генерируем сетку вершин с учетом перекрытия
    for z in 0..size + overlap {
        for x in 0..size + overlap {
            let pixel_x = start_x + x;
            let pixel_z = start_z + z;

            // Получаем значение высоты из карты высот
            let height_value = heightmap.get_pixel(pixel_x, pixel_z).0[0];
            // Нормализуем высоту
            let height = (height_value as f32 / 255.0) * max_height;

            // Создаем вершины только для точек выше порога
            if height > 0.0 {
                has_valid_height = true;

                // Добавляем вершину в списки
                vertices.push([x as f32, height / 2.0, z as f32]);

                // Временная нормаль (будет пересчитана позже)
                normals.push([0.0, 1.0, 0.0]);

                // UV-координаты
                uvs.push([x as f32 / size as f32, z as f32 / size as f32]);

                // Сохраняем индекс вершины в нашем массиве
                vertex_indices[z as usize][x as usize] = Some((vertices.len() - 1) as u32);
            }
        }
    }

    // Если нет точек выше порога, возвращаем None
    if !has_valid_height {
        return None;
    }

    // Генерируем треугольники
    for z in 0..size + overlap - 1 {
        for x in 0..size + overlap - 1 {
            // Проверяем наличие вершин в углах квадрата
            let top_left = vertex_indices[z as usize][x as usize];
            let top_right = vertex_indices[z as usize][(x + 1) as usize];
            let bottom_left = vertex_indices[(z + 1) as usize][x as usize];
            let bottom_right = vertex_indices[(z + 1) as usize][(x + 1) as usize];

            // Генерируем треугольники для каждого квадрата, где есть достаточно вершин
            match (top_left, top_right, bottom_left, bottom_right) {
                (Some(tl), Some(tr), Some(bl), Some(br)) => {
                    // Два треугольника с правильной ориентацией (по часовой стрелке)
                    indices.extend_from_slice(&[tl, bl, tr]); // Первый треугольник
                    indices.extend_from_slice(&[tr, bl, br]); // Второй треугольник
                },
                (Some(tl), Some(tr), Some(bl), None) => {
                    // Один треугольник
                    indices.extend_from_slice(&[tl, bl, tr]);
                },
                (Some(tl), Some(tr), None, Some(br)) => {
                    // Один треугольник
                    indices.extend_from_slice(&[tl, tr, br]);
                },
                (Some(tl), None, Some(bl), Some(br)) => {
                    // Один треугольник
                    indices.extend_from_slice(&[tl, bl, br]);
                },
                (None, Some(tr), Some(bl), Some(br)) => {
                    // Один треугольник
                    indices.extend_from_slice(&[tr, bl, br]);
                },
                _ => {
                    // Недостаточно вершин для треугольника
                    continue;
                }
            }
        }
    }

    // Если не удалось создать ни одного треугольника, возвращаем None
    if indices.is_empty() {
        return None;
    }

    // Создаем меш из наших данных
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));

    // Вычисляем нормали для правильного освещения
    mesh.compute_smooth_normals();

    Some(mesh)
}
