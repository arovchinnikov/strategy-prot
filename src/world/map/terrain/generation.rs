use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use image::GrayImage;
use noise::{NoiseFn, Perlin};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::HashSet;

pub fn generate_terrain_mesh(
    start_x: u32,
    start_z: u32,
    heightmap: &GrayImage,
    overlap: u32,
    seed: u32, // Сид для генерации шума
) -> Option<Mesh> {
    let size = 128;
    let max_height = 100.0;

    // Инициализация генераторов случайных чисел и шума
    let mut rng = StdRng::seed_from_u64(seed as u64);
    let noise = Perlin::new(seed);

    // Настройки для обработки границ
    let boundary_variation = 0.4; // Уменьшаем максимальное смещение для более плавных границ
    let noise_scale = 0.05; // Уменьшаем масштаб шума для более плавных изменений

    // Проверяем, достаточно ли большая карта высот, учитывая перекрытие
    if start_x + size + overlap > heightmap.width() || start_z + size + overlap > heightmap.height() {
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

    // Определяем, какие ячейки являются граничными
    let mut boundary_cells: HashSet<(usize, usize)> = HashSet::new();
    let mut near_boundary_cells: HashSet<(usize, usize)> = HashSet::new();

    // Собираем высоты и отмечаем ненулевые клетки
    let mut active_cells: HashSet<(usize, usize)> = HashSet::new();
    for z in 0..points_height {
        for x in 0..points_width {
            let heightmap_x = start_x + x;
            let heightmap_z = start_z + z;

            // Получаем высоту из карты высот (без изменений!)
            let pixel = heightmap.get_pixel(heightmap_x, heightmap_z);
            let height = if pixel[0] > 0 {
                let h = (pixel[0] as f32 / 255.0) * max_height;
                if h > 0.0 {
                    has_non_zero_height = true;
                    active_cells.insert((x as usize, z as usize));
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

    // Определяем граничные ячейки (где есть соседи с нулевой высотой)
    for &(x, z) in &active_cells {
        // Проверяем соседей в пределах 2 клеток для более плавного перехода
        let mut is_boundary = false;

        // Прямые соседи
        let neighbors = [
            (x.saturating_sub(1), z),
            (x + 1, z),
            (x, z.saturating_sub(1)),
            (x, z + 1),
        ];

        for (nx, nz) in neighbors {
            if nx < points_width as usize && nz < points_height as usize {
                if heights[nz][nx] == 0.0 {
                    is_boundary = true;
                    boundary_cells.insert((x, z));
                    break;
                }
            }
        }

        // Если это граничная ячейка, отмечаем соседей для плавного перехода
        if is_boundary {
            // Добавляем соседей на расстоянии 2-3 клеток к "околограничным" ячейкам
            for dz in -2..=2 {
                for dx in -2..=2 {
                    let nx = x as isize + dx;
                    let nz = z as isize + dz;

                    if nx >= 0 && nz >= 0 &&
                        nx < points_width as isize && nz < points_height as isize &&
                        heights[nz as usize][nx as usize] > 0.0 {
                        // Расстояние от ячейки до границы определяет силу эффекта
                        near_boundary_cells.insert((nx as usize, nz as usize));
                    }
                }
            }
        }
    }

    // Создаем вершины с модификациями для граничных клеток
    let mut vertex_id_map: Vec<Vec<Option<u32>>> = vec![vec![None; points_width as usize]; points_height as usize];
    let mut vertex_counter: u32 = 0;

    // Функция для получения шума в определенной точке
    let get_noise = |x: f32, z: f32| -> f32 {
        noise.get([x as f64 * noise_scale, z as f64 * noise_scale]) as f32
    };

    // Создаем вершины для активных ячеек
    for z in 0..points_height {
        for x in 0..points_width {
            let x_usize = x as usize;
            let z_usize = z as usize;

            // Пропускаем создание вершин в клетках с нулевой высотой
            if heights[z_usize][x_usize] == 0.0 {
                continue;
            }

            let mut pos_x = x as f32;
            let mut pos_z = z as f32;
            let height = heights[z_usize][x_usize]; // Сохраняем оригинальную высоту из карты высот

            // Определяем, насколько ячейка близка к границе
            let is_boundary = boundary_cells.contains(&(x_usize, z_usize));
            let is_near_boundary = near_boundary_cells.contains(&(x_usize, z_usize));

            // Вычисляем коэффициент влияния в зависимости от расстояния до границы
            let boundary_factor = if is_boundary {
                1.0
            } else if is_near_boundary {
                // Определяем расстояние до ближайшей граничной ячейки
                let mut min_dist = f32::MAX;
                for &(bx, bz) in &boundary_cells {
                    let dx = bx as f32 - x_usize as f32;
                    let dz = bz as f32 - z_usize as f32;
                    let dist = (dx*dx + dz*dz).sqrt();
                    min_dist = min_dist.min(dist);
                }

                // Плавное затухание влияния с расстоянием (экспоненциальное затухание)
                (-(min_dist * 0.7)).exp().min(0.8)
            } else {
                0.0
            };

            if boundary_factor > 0.0 {
                // Используем шум Перлина для более естественных смещений,
                // основанный на глобальных координатах для плавности между чанками
                let global_x = start_x as f32 + x as f32;
                let global_z = start_z as f32 + z as f32;

                // Получаем шум для разных смещений, чтобы избежать повторения
                let noise_val_x = get_noise(global_x, global_z);
                let noise_val_z = get_noise(global_x + 100.0, global_z + 100.0);

                // Смещаем только внутрь области (не в сторону нулевых высот)
                let mut can_move_left = x_usize > 0 && heights[z_usize][x_usize - 1] > 0.0;
                let mut can_move_right = x_usize < points_width as usize - 1 && heights[z_usize][x_usize + 1] > 0.0;
                let mut can_move_up = z_usize > 0 && heights[z_usize - 1][x_usize] > 0.0;
                let mut can_move_down = z_usize < points_height as usize - 1 && heights[z_usize + 1][x_usize] > 0.0;

                // Дополнительная проверка для диагональных направлений
                if !can_move_left && !can_move_up && x_usize > 0 && z_usize > 0 &&
                    heights[z_usize - 1][x_usize - 1] > 0.0 {
                    can_move_left = true;
                    can_move_up = true;
                }

                if !can_move_right && !can_move_up && x_usize < points_width as usize - 1 && z_usize > 0 &&
                    heights[z_usize - 1][x_usize + 1] > 0.0 {
                    can_move_right = true;
                    can_move_up = true;
                }

                if !can_move_left && !can_move_down && x_usize > 0 && z_usize < points_height as usize - 1 &&
                    heights[z_usize + 1][x_usize - 1] > 0.0 {
                    can_move_left = true;
                    can_move_down = true;
                }

                if !can_move_right && !can_move_down && x_usize < points_width as usize - 1 && z_usize < points_height as usize - 1 &&
                    heights[z_usize + 1][x_usize + 1] > 0.0 {
                    can_move_right = true;
                    can_move_down = true;
                }

                // Вычисляем базовое смещение на основе шума
                let base_offset_x = noise_val_x * boundary_variation;
                let base_offset_z = noise_val_z * boundary_variation;

                // Применяем смещения только в допустимых направлениях с учетом затухания
                let offset_x = if can_move_left && can_move_right {
                    base_offset_x * boundary_factor
                } else if can_move_left {
                    -rng.gen_range(0.0..boundary_variation) * boundary_factor
                } else if can_move_right {
                    rng.gen_range(0.0..boundary_variation) * boundary_factor
                } else {
                    0.0
                };

                let offset_z = if can_move_up && can_move_down {
                    base_offset_z * boundary_factor
                } else if can_move_up {
                    -rng.gen_range(0.0..boundary_variation) * boundary_factor
                } else if can_move_down {
                    rng.gen_range(0.0..boundary_variation) * boundary_factor
                } else {
                    0.0
                };

                pos_x += offset_x;
                pos_z += offset_z;
            }

            vertices.push([pos_x, height, pos_z]); // Высота всегда точно соответствует карте высот
            uvs.push([x as f32 / (size + overlap) as f32, z as f32 / (size + overlap) as f32]);
            normals.push([0.0, 1.0, 0.0]); // Временные нормали

            vertex_id_map[z_usize][x_usize] = Some(vertex_counter);
            vertex_counter += 1;
        }
    }

    // Создаем треугольники с использованием марширующих квадратов для граничных областей
    for z in 0..size + overlap {
        for x in 0..size + overlap {
            let z_usize = z as usize;
            let x_usize = x as usize;

            let h_tl = heights[z_usize][x_usize];
            let h_tr = heights[z_usize][x_usize + 1];
            let h_bl = heights[z_usize + 1][x_usize];
            let h_br = heights[z_usize + 1][x_usize + 1];

            // Пропускаем полностью пустые квадраты
            if h_tl == 0.0 && h_tr == 0.0 && h_bl == 0.0 && h_br == 0.0 {
                continue;
            }

            // Получаем индексы вершин, если они существуют
            let top_left = vertex_id_map[z_usize][x_usize];
            let top_right = vertex_id_map[z_usize][x_usize + 1];
            let bottom_left = vertex_id_map[z_usize + 1][x_usize];
            let bottom_right = vertex_id_map[z_usize + 1][x_usize + 1];

            // Если все вершины существуют, то создаем стандартный квадрат
            if let (Some(tl), Some(tr), Some(bl), Some(br)) = (top_left, top_right, bottom_left, bottom_right) {
                // Первый треугольник (против часовой стрелки)
                indices.push(tl);
                indices.push(bl);
                indices.push(tr);

                // Второй треугольник (против часовой стрелки)
                indices.push(tr);
                indices.push(bl);
                indices.push(br);
            }
            // Если не все вершины существуют, и это граничная ячейка, то применяем марширующие квадраты
            else if h_tl > 0.0 || h_tr > 0.0 || h_bl > 0.0 || h_br > 0.0 {
                // Определяем тип случая марширующих квадратов
                let case_index = ((h_tl > 0.0) as u8) |
                    ((h_tr > 0.0) as u8) << 1 |
                    ((h_br > 0.0) as u8) << 2 |
                    ((h_bl > 0.0) as u8) << 3;

                // Обрабатываем различные случаи марширующих квадратов
                match case_index {
                    // Один угол (треугольник)
                    // Пропускаем случаи с одной вершиной для более плавных границ
                    1 | 2 | 4 | 8 => {},

                    // Два смежных угла (треугольник)
                    3 => if let (Some(tl), Some(tr)) = (top_left, top_right) {
                        if let Some(bl) = bottom_left {
                            indices.push(tl);
                            indices.push(bl);
                            indices.push(tr);
                        }
                    },
                    6 => if let (Some(tr), Some(br)) = (top_right, bottom_right) {
                        if let Some(tl) = top_left {
                            indices.push(tl);
                            indices.push(br);
                            indices.push(tr);
                        }
                    },
                    12 => if let (Some(bl), Some(br)) = (bottom_left, bottom_right) {
                        if let Some(tl) = top_left {
                            indices.push(tl);
                            indices.push(bl);
                            indices.push(br);
                        }
                    },
                    9 => if let (Some(tl), Some(bl)) = (top_left, bottom_left) {
                        if let Some(br) = bottom_right {
                            indices.push(tl);
                            indices.push(bl);
                            indices.push(br);
                        }
                    },

                    // Противоположные углы - обрабатываем более аккуратно для плавных границ
                    5 => if let (Some(tl), Some(br)) = (top_left, bottom_right) {
                        // Создаем один треугольник, выбирая наименее экстремальное соединение
                        if h_tl > h_br {
                            indices.push(tl);
                            indices.push(br);
                            indices.push(tl); // Дублируем вершину, т.к. нет третьей
                        } else {
                            indices.push(tl);
                            indices.push(br);
                            indices.push(br); // Дублируем вершину, т.к. нет третьей
                        }
                    },
                    10 => if let (Some(tr), Some(bl)) = (top_right, bottom_left) {
                        // Создаем один треугольник, выбирая наименее экстремальное соединение
                        if h_tr > h_bl {
                            indices.push(tr);
                            indices.push(bl);
                            indices.push(tr); // Дублируем вершину, т.к. нет третьей
                        } else {
                            indices.push(tr);
                            indices.push(bl);
                            indices.push(bl); // Дублируем вершину, т.к. нет третьей
                        }
                    },

                    // Три угла (два треугольника)
                    7 => if let (Some(tl), Some(tr), Some(br)) = (top_left, top_right, bottom_right) {
                        if let Some(bl) = bottom_left {
                            // Первый треугольник
                            indices.push(tl);
                            indices.push(bl);
                            indices.push(tr);

                            // Второй треугольник
                            indices.push(tr);
                            indices.push(bl);
                            indices.push(br);
                        } else {
                            // Если нет нижней левой вершины, делаем один треугольник
                            indices.push(tl);
                            indices.push(br);
                            indices.push(tr);
                        }
                    },
                    11 => if let (Some(tl), Some(tr), Some(bl)) = (top_left, top_right, bottom_left) {
                        if let Some(br) = bottom_right {
                            // Первый треугольник
                            indices.push(tl);
                            indices.push(bl);
                            indices.push(tr);

                            // Второй треугольник
                            indices.push(tr);
                            indices.push(bl);
                            indices.push(br);
                        } else {
                            // Если нет нижней правой вершины, делаем один треугольник
                            indices.push(tl);
                            indices.push(bl);
                            indices.push(tr);
                        }
                    },
                    13 => if let (Some(tl), Some(bl), Some(br)) = (top_left, bottom_left, bottom_right) {
                        if let Some(tr) = top_right {
                            // Первый треугольник
                            indices.push(tl);
                            indices.push(bl);
                            indices.push(tr);

                            // Второй треугольник
                            indices.push(tr);
                            indices.push(bl);
                            indices.push(br);
                        } else {
                            // Если нет верхней правой вершины, делаем один треугольник
                            indices.push(tl);
                            indices.push(bl);
                            indices.push(br);
                        }
                    },
                    14 => if let (Some(tr), Some(bl), Some(br)) = (top_right, bottom_left, bottom_right) {
                        if let Some(tl) = top_left {
                            // Первый треугольник
                            indices.push(tl);
                            indices.push(bl);
                            indices.push(tr);

                            // Второй треугольник
                            indices.push(tr);
                            indices.push(bl);
                            indices.push(br);
                        } else {
                            // Если нет верхней левой вершины, делаем один треугольник
                            indices.push(tr);
                            indices.push(bl);
                            indices.push(br);
                        }
                    },

                    // Все четыре угла (стандартный квадрат) - этот случай должен быть обработан выше
                    15 => {},

                    // Нет угловых вершин - ничего не делаем
                    0 => {},

                    _ => {}
                }
            }
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
