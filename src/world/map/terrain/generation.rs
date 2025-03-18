use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use image::GrayImage;

const VOID_HEIGHT: u8 = 0;

pub fn generate_terrain_mesh(
    start_x: u32,
    start_z: u32,
    heightmap: &GrayImage,
) -> Option<Mesh> {
    let chunk_size = 128;
    let end_x = (start_x + chunk_size).min(heightmap.width());
    let end_z = (start_z + chunk_size).min(heightmap.height());

    // Проверяем, что размер чанка не нулевой
    if end_x <= start_x || end_z <= start_z {
        return None;
    }

    // Вычисляем размер чанка
    let width = (end_x - start_x) as usize;
    let depth = (end_z - start_z) as usize;

    // Быстрая проверка на пустой чанк
    let mut has_terrain = false;
    for z in start_z..end_z {
        for x in start_x..end_x {
            if heightmap.get_pixel(x, z)[0] != VOID_HEIGHT {
                has_terrain = true;
                break;
            }
        }
        if has_terrain {
            break;
        }
    }

    // Если чанк полностью пустой, возвращаем None
    if !has_terrain {
        return None;
    }

    // Подготавливаем векторы для хранения данных меша
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    // Масштабный фактор для высоты
    let height_scale = 0.1;

    // Создаем квадраты поверхности для каждого пикселя на карте высот
    for z in start_z..end_z {
        for x in start_x..end_x {
            // Пропускаем пустые области
            if heightmap.get_pixel(x, z)[0] == VOID_HEIGHT {
                continue;
            }

            // Вычисляем относительные координаты внутри чанка
            let local_x = (x - start_x) as f32;
            let local_z = (z - start_z) as f32;

            // Получаем высоту текущего пикселя
            let current_height = heightmap.get_pixel(x, z)[0] as f32 * height_scale;

            // Индекс первой вершины этого квадрата
            let vertex_start_index = positions.len() as u32;

            // Добавляем четыре вершины для текущего квадрата
            // Важно: все вершины имеют одинаковую высоту, предотвращая "свисание"
            // Верхний левый угол
            positions.push([local_x, current_height, local_z]);
            uvs.push([local_x / width as f32, local_z / depth as f32]);
            normals.push([0.0, 1.0, 0.0]); // будем перерасчитывать позже

            // Верхний правый угол
            positions.push([local_x + 1.0, current_height, local_z]);
            uvs.push([(local_x + 1.0) / width as f32, local_z / depth as f32]);
            normals.push([0.0, 1.0, 0.0]);

            // Нижний правый угол
            positions.push([local_x + 1.0, current_height, local_z + 1.0]);
            uvs.push([(local_x + 1.0) / width as f32, (local_z + 1.0) / depth as f32]);
            normals.push([0.0, 1.0, 0.0]);

            // Нижний левый угол
            positions.push([local_x, current_height, local_z + 1.0]);
            uvs.push([local_x / width as f32, (local_z + 1.0) / depth as f32]);
            normals.push([0.0, 1.0, 0.0]);

            // Добавляем индексы для двух треугольников квадрата
            // Первый треугольник (верхний левый - верхний правый - нижний правый)
            indices.push(vertex_start_index + 1); // верхний правый
            indices.push(vertex_start_index);     // верхний левый
            indices.push(vertex_start_index + 2); // нижний правый

            // Второй треугольник (верхний левый - нижний правый - нижний левый)
            indices.push(vertex_start_index);     // верхний левый
            indices.push(vertex_start_index + 3); // нижний левый
            indices.push(vertex_start_index + 2); // нижний правый
        }
    }

    // Если нет индексов, значит нет поверхности для отображения
    if indices.is_empty() {
        return None;
    }

    // Вычисляем нормали для треугольников
    // Сначала инициализируем все нормали нулевыми векторами
    for i in 0..normals.len() {
        normals[i] = [0.0, 0.0, 0.0];
    }

    // Для каждого треугольника вычисляем нормаль и добавляем ее ко всем вершинам треугольника
    for i in (0..indices.len()).step_by(3) {
        if i + 2 >= indices.len() {
            continue;
        }

        let i0 = indices[i] as usize;
        let i1 = indices[i + 1] as usize;
        let i2 = indices[i + 2] as usize;

        let p0 = positions[i0];
        let p1 = positions[i1];
        let p2 = positions[i2];

        // Вычисляем два вектора треугольника
        let v1 = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
        let v2 = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];

        // Вычисляем нормаль как векторное произведение
        let normal = [
            v1[1] * v2[2] - v1[2] * v2[1],
            v1[2] * v2[0] - v1[0] * v2[2],
            v1[0] * v2[1] - v1[1] * v2[0]
        ];

        // Добавляем вычисленную нормаль к нормалям вершин треугольника
        for idx in [i0, i1, i2] {
            normals[idx][0] += normal[0];
            normals[idx][1] += normal[1];
            normals[idx][2] += normal[2];
        }
    }

    // Нормализуем все нормали
    for i in 0..normals.len() {
        let n = normals[i];
        let length = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();

        if length > 0.0001 {
            normals[i] = [n[0] / length, n[1] / length, n[2] / length];
        } else {
            // Если нормаль близка к нулю, устанавливаем её по умолчанию вверх
            normals[i] = [0.0, 1.0, 0.0];
        }
    }

    // Создаем меш
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));

    Some(mesh)
}
