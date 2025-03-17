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
    // Применяем subdivision-подобное сглаживание к граничным вершинам
    smooth_border_with_subdivision(&mut vertices, &heights, points_width, points_height);

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));

    // Вычисляем нормали для правильного освещения
    mesh.compute_smooth_normals();

    Some(mesh)
}

// Функция сглаживания с эффектом subdivision для граничных вершин
fn smooth_border_with_subdivision(
    vertices: &mut Vec<[f32; 3]>,
    heights: &Vec<Vec<f32>>,
    points_width: u32,
    points_height: u32
) {
    // Шаг 1: Определяем граничные вершины
    let mut is_border = vec![false; vertices.len()];

    for z in 1..points_height - 1 {
        for x in 1..points_width - 1 {
            let idx = (z * points_width + x) as usize;
            let current_height = heights[z as usize][x as usize];

            if current_height == 0.0 {
                continue;
            }

            // Проверяем 8 соседей
            let neighbors = [
                (x-1, z-1), (x, z-1), (x+1, z-1),
                (x-1, z),               (x+1, z),
                (x-1, z+1), (x, z+1), (x+1, z+1)
            ];

            for &(nx, nz) in &neighbors {
                if nx < points_width && nz < points_height {
                    let neighbor_height = heights[nz as usize][nx as usize];

                    if neighbor_height == 0.0 {
                        is_border[idx] = true;
                        break;
                    }
                }
            }
        }
    }

    // Шаг 2: Применяем итеративное сглаживание в стиле Subdivision
    const ITERATIONS: usize = 7;  // Количество итераций subdivision
    const SMOOTHING_FACTOR: f32 = 0.2;  // Фактор сглаживания

    for _ in 0..ITERATIONS {
        let current_vertices = vertices.clone();

        for z in 1..points_height - 1 {
            for x in 1..points_width - 1 {
                let idx = (z * points_width + x) as usize;

                // Обрабатываем только граничные вершины с ненулевой высотой
                if !is_border[idx] || heights[z as usize][x as usize] == 0.0 {
                    continue;
                }

                // Ищем соседние граничные вершины
                let mut valid_neighbors = Vec::new();

                // Проверяем соседей по 4 основным направлениям для более прямых связей
                let direct_neighbors = [
                    (x, z-1), (x+1, z), (x, z+1), (x-1, z)
                ];

                for &(nx, nz) in &direct_neighbors {
                    if nx < points_width && nz < points_height {
                        let n_idx = (nz * points_width + nx) as usize;
                        let n_height = heights[nz as usize][nx as usize];

                        if n_height > 0.0 && is_border[n_idx] {
                            valid_neighbors.push(n_idx);
                        }
                    }
                }

                // Если найдено менее 2 соседей, проверяем также диагональные соседи
                if valid_neighbors.len() < 2 {
                    let diagonal_neighbors = [
                        (x-1, z-1), (x+1, z-1),
                        (x-1, z+1), (x+1, z+1)
                    ];

                    for &(nx, nz) in &diagonal_neighbors {
                        if nx < points_width && nz < points_height {
                            let n_idx = (nz * points_width + nx) as usize;
                            let n_height = heights[nz as usize][nx as usize];

                            if n_height > 0.0 && is_border[n_idx] {
                                valid_neighbors.push(n_idx);
                            }
                        }
                    }
                }

                // Применяем subdivision-подобное правило сглаживания
                if valid_neighbors.len() >= 2 {
                    let mut new_x = 0.0;
                    let mut new_z = 0.0;

                    // Вычисляем новую позицию как среднее соседних вершин + начальная вершина
                    for &n_idx in &valid_neighbors {
                        new_x += current_vertices[n_idx][0];
                        new_z += current_vertices[n_idx][2];
                    }

                    new_x = new_x / valid_neighbors.len() as f32;
                    new_z = new_z / valid_neighbors.len() as f32;

                    // Интерполируем с учетом коэффициента сглаживания
                    vertices[idx][0] = current_vertices[idx][0] * (1.0 - SMOOTHING_FACTOR) + new_x * SMOOTHING_FACTOR;
                    vertices[idx][2] = current_vertices[idx][2] * (1.0 - SMOOTHING_FACTOR) + new_z * SMOOTHING_FACTOR;
                    // Высота (Y) остается неизменной
                }
            }
        }
    }

    // Шаг 3: Финальный проход для более плавного перехода между соседними вершинами
    let final_vertices = vertices.clone();

    for z in 1..points_height - 1 {
        for x in 1..points_width - 1 {
            let idx = (z * points_width + x) as usize;

            if !is_border[idx] || heights[z as usize][x as usize] == 0.0 {
                continue;
            }

            // Используем Catmull-Clark-подобную схему для финального сглаживания
            let mut count = 0;
            let mut sum_x = 0.0;
            let mut sum_z = 0.0;

            // Проверяем в радиусе 2 для лучшего эффекта сглаживания
            for dz in -2..=2 {
                for dx in -2..=2 {
                    if dx == 0 && dz == 0 {
                        continue;
                    }

                    let nx = x as i32 + dx;
                    let nz = z as i32 + dz;

                    if nx >= 0 && nx < points_width as i32 && nz >= 0 && nz < points_height as i32 {
                        let n_idx = (nz as u32 * points_width + nx as u32) as usize;

                        if is_border[n_idx] && heights[nz as usize][nx as usize] > 0.0 {
                            sum_x += final_vertices[n_idx][0];
                            sum_z += final_vertices[n_idx][2];
                            count += 1;
                        }
                    }
                }
            }

            if count > 0 {
                // Интерполируем с меньшим коэффициентом для финального прохода
                const FINAL_FACTOR: f32 = 0.3;
                let avg_x = sum_x / count as f32;
                let avg_z = sum_z / count as f32;

                vertices[idx][0] = final_vertices[idx][0] * (1.0 - FINAL_FACTOR) + avg_x * FINAL_FACTOR;
                vertices[idx][2] = final_vertices[idx][2] * (1.0 - FINAL_FACTOR) + avg_z * FINAL_FACTOR;
            }
        }
    }
}
