use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use image::GrayImage;
use std::collections::{HashMap, HashSet};

const VOID_HEIGHT: u8 = 0;

pub fn generate_terrain_mesh(
    start_x: u32,
    start_z: u32,
    heightmap: &GrayImage,
) -> Option<Mesh> {
    let chunk_size = 128;

    // Добавляем перекрытие в 1 пиксель для правильного стыка чанков
    // Но учитываем границы heightmap
    let expanded_start_x = start_x.saturating_sub(1);
    let expanded_start_z = start_z.saturating_sub(1);
    let expanded_end_x = (start_x + chunk_size + 1).min(heightmap.width());
    let expanded_end_z = (start_z + chunk_size + 1).min(heightmap.height());

    // Проверяем, что размер чанка не нулевой
    if expanded_end_x <= expanded_start_x || expanded_end_z <= expanded_start_z {
        return None;
    }

    // Вычисляем размер расширенного чанка
    let expanded_width = (expanded_end_x - expanded_start_x) as usize;
    let expanded_depth = (expanded_end_z - expanded_start_z) as usize;

    // Быстрая проверка на пустой чанк (только для области без перекрытия)
    let mut has_terrain = false;
    for z in start_z..start_z + chunk_size.min(heightmap.height() - start_z) {
        for x in start_x..start_x + chunk_size.min(heightmap.width() - start_x) {
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

    // Хеш-карта для отслеживания уже созданных вершин
    // Ключ: (x, z) координаты вершины, Значение: индекс вершины в массиве positions
    let mut vertex_map: HashMap<(u32, u32), u32> = HashMap::new();

    // Масштабный фактор для высоты
    let height_scale = 0.3;

    // Первый проход: создаем сетку вершин только для непустых пикселей в расширенной области
    // Для каждого непустого пикселя создаем вершину в его верхнем левом углу
    for z in expanded_start_z..expanded_end_z + 1 {
        for x in expanded_start_x..expanded_end_x + 1 {
            // Определяем, нужна ли вершина в этой позиции (x, z)
            let is_needed =
                // Проверяем, есть ли непустой пиксель слева-сверху от этой вершины
                (x > expanded_start_x && z > expanded_start_z &&
                    x - 1 < heightmap.width() && z - 1 < heightmap.height() &&
                    heightmap.get_pixel(x-1, z-1)[0] != VOID_HEIGHT) ||
                    // Проверяем, есть ли непустой пиксель справа-сверху от этой вершины
                    (x < expanded_end_x && z > expanded_start_z &&
                        x < heightmap.width() && z - 1 < heightmap.height() &&
                        heightmap.get_pixel(x, z-1)[0] != VOID_HEIGHT) ||
                    // Проверяем, есть ли непустой пиксель слева-снизу от этой вершины
                    (x > expanded_start_x && z < expanded_end_z &&
                        x - 1 < heightmap.width() && z < heightmap.height() &&
                        heightmap.get_pixel(x-1, z)[0] != VOID_HEIGHT) ||
                    // Проверяем, есть ли непустой пиксель справа-снизу от этой вершины
                    (x < expanded_end_x && z < expanded_end_z &&
                        x < heightmap.width() && z < heightmap.height() &&
                        heightmap.get_pixel(x, z)[0] != VOID_HEIGHT);

            if is_needed {
                // Определяем высоту вершины на основе окружающих пикселей
                let mut height_sum = 0.0;
                let mut count = 0;

                // Проверяем высоты всех соседних непустых пикселей
                if x > expanded_start_x && z > expanded_start_z &&
                    x - 1 < heightmap.width() && z - 1 < heightmap.height() &&
                    heightmap.get_pixel(x-1, z-1)[0] != VOID_HEIGHT {
                    height_sum += heightmap.get_pixel(x-1, z-1)[0] as f32;
                    count += 1;
                }
                if x < expanded_end_x && z > expanded_start_z &&
                    x < heightmap.width() && z - 1 < heightmap.height() &&
                    heightmap.get_pixel(x, z-1)[0] != VOID_HEIGHT {
                    height_sum += heightmap.get_pixel(x, z-1)[0] as f32;
                    count += 1;
                }
                if x > expanded_start_x && z < expanded_end_z &&
                    x - 1 < heightmap.width() && z < heightmap.height() &&
                    heightmap.get_pixel(x-1, z)[0] != VOID_HEIGHT {
                    height_sum += heightmap.get_pixel(x-1, z)[0] as f32;
                    count += 1;
                }
                if x < expanded_end_x && z < expanded_end_z &&
                    x < heightmap.width() && z < heightmap.height() &&
                    heightmap.get_pixel(x, z)[0] != VOID_HEIGHT {
                    height_sum += heightmap.get_pixel(x, z)[0] as f32;
                    count += 1;
                }

                // Вычисляем среднюю высоту для вершины
                let height = if count > 0 {
                    (height_sum / count as f32) * height_scale
                } else {
                    0.0 // По умолчанию, если нет соседних пикселей (не должно происходить)
                };

                // Вычисляем относительные координаты внутри чанка
                // Важно: координаты относительно оригинального начала чанка (start_x, start_z), а не расширенного
                let local_x = (x as i32 - start_x as i32) as f32;
                let local_z = (z as i32 - start_z as i32) as f32;

                // Добавляем вершину
                let vertex_index = positions.len() as u32;
                positions.push([local_x, height, local_z]);
                normals.push([0.0, 1.0, 0.0]); // Будет пересчитано позже
                uvs.push([
                    (x - expanded_start_x) as f32 / expanded_width as f32,
                    (z - expanded_start_z) as f32 / expanded_depth as f32
                ]);

                // Сохраняем индекс вершины в карте
                vertex_map.insert((x, z), vertex_index);
            }
        }
    }

    // Второй проход: создаем треугольники для всей расширенной области
    // Это важно для правильного расчета нормалей на границах
    let mut all_indices: Vec<u32> = Vec::new();

    for z in expanded_start_z..expanded_end_z {
        for x in expanded_start_x..expanded_end_x {
            // Пропускаем пустые пиксели
            if x >= heightmap.width() || z >= heightmap.height() ||
                heightmap.get_pixel(x, z)[0] == VOID_HEIGHT {
                continue;
            }

            // Получаем индексы четырех вершин квадрата
            // Если эти вершины уже созданы, используем их индексы
            if let (Some(&top_left), Some(&top_right), Some(&bottom_right), Some(&bottom_left)) = (
                vertex_map.get(&(x, z)),
                vertex_map.get(&(x+1, z)),
                vertex_map.get(&(x+1, z+1)),
                vertex_map.get(&(x, z+1))
            ) {
                // Добавляем индексы для двух треугольников (один квадрат)
                // Первый треугольник (верхний правый - верхний левый - нижний правый)
                all_indices.push(top_right);    // верхний правый
                all_indices.push(top_left);     // верхний левый
                all_indices.push(bottom_right); // нижний правый

                // Второй треугольник (верхний левый - нижний левый - нижний правый)
                all_indices.push(top_left);     // верхний левый
                all_indices.push(bottom_left);  // нижний левый
                all_indices.push(bottom_right); // нижний правый

                // Если этот квадрат принадлежит оригинальному чанку (не из перекрытия),
                // то добавляем его индексы в итоговый меш
                if x >= start_x && x < start_x + chunk_size && z >= start_z && z < start_z + chunk_size {
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

    // Если нет индексов, значит нет поверхности для отображения
    if indices.is_empty() {
        return None;
    }

    // Вычисляем нормали для треугольников всей расширенной области
    // Сначала инициализируем все нормали нулевыми векторами
    for i in 0..normals.len() {
        normals[i] = [0.0, 0.0, 0.0];
    }

    // Для каждого треугольника вычисляем нормаль и добавляем ее ко всем вершинам треугольника
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

    generate_border(
        heightmap,
        start_x,
        start_z,
        start_x + chunk_size,
        start_z + chunk_size
    );

    // Создаем меш с вершинами и индексами только для оригинального чанка
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));

    Some(mesh)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Point {
    x: u32,
    y: u32,
}

#[derive(Debug)]
struct BorderPixel {
    position: Point,
    neighbors: Vec<Point>,
}

// Соседи по 4 сторонам (не включая диагональные)
const DIRECTIONS: [(i32, i32); 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];
// Все 8 направлений для поиска следующего пикселя границы (по часовой стрелке)
const MOORE_DIRECTIONS: [(i32, i32); 8] = [
    (1, 0),   // вправо
    (1, 1),   // вправо-вниз
    (0, 1),   // вниз
    (-1, 1),  // влево-вниз
    (-1, 0),  // влево
    (-1, -1), // влево-вверх
    (0, -1),  // вверх
    (1, -1),  // вправо-вверх
];

fn generate_border(
    heightmap: &GrayImage,
    start_x: u32,
    start_z: u32,
    end_x: u32,
    end_z: u32,
) {
    info!("Поиск границ для чанка: ({}, {}) -> ({}, {})", start_x, start_z, end_x, end_z);

    // Находим все граничные пиксели в пределах чанка
    let mut border_pixels: Vec<Point> = Vec::new();

    // Сначала находим все пиксели, которые находятся на границе
    for z in start_z..end_z {
        for x in start_x..end_x {
            // Пропускаем, если пиксель выходит за границы карты высот
            if x >= heightmap.width() || z >= heightmap.height() {
                continue;
            }

            // Если пиксель не пустой (высота > 0)
            if heightmap.get_pixel(x, z)[0] != VOID_HEIGHT {
                // Проверяем, есть ли у него сосед с высотой 0
                let mut is_border = false;

                for (dx, dz) in DIRECTIONS.iter() {
                    let nx = x as i32 + dx;
                    let nz = z as i32 + dz;

                    // Проверяем, что сосед в пределах границ карты высот
                    if nx >= 0 && nx < heightmap.width() as i32 &&
                        nz >= 0 && nz < heightmap.height() as i32 {
                        // Если сосед пустой, текущий пиксель - граничный
                        if heightmap.get_pixel(nx as u32, nz as u32)[0] == VOID_HEIGHT {
                            is_border = true;
                            break;
                        }
                    } else {
                        // Если сосед за пределами карты, считаем текущий пиксель граничным
                        is_border = true;
                        break;
                    }
                }

                if is_border {
                    border_pixels.push(Point { x, y: z });
                }
            }
        }
    }

    info!("Найдено {} граничных пикселей в чанке", border_pixels.len());

    // Множество для уже обработанных пикселей
    let mut processed = HashSet::new();

    // Обходим каждую границу
    let mut borders_count = 0;

    while !border_pixels.is_empty() {
        // Выбираем начальный пиксель, который не был обработан
        let mut start_idx = None;
        for (idx, pixel) in border_pixels.iter().enumerate() {
            if !processed.contains(pixel) {
                start_idx = Some(idx);
                break;
            }
        }

        // Если все пиксели обработаны, выходим из цикла
        if start_idx.is_none() {
            break;
        }

        borders_count += 1;
        info!("Обход границы #{}", borders_count);

        // Получаем стартовую точку
        let start_point = border_pixels[start_idx.unwrap()];
        let mut boundary = Vec::new();

        // Начинаем обход с текущего пикселя
        let mut current = start_point;
        let mut prev_direction = 0; // начальное направление - вправо (индекс 0 в MOORE_DIRECTIONS)

        // Для предотвращения бесконечных циклов
        let mut max_iterations = border_pixels.len() * 2;

        loop {
            // Если достигнут лимит итераций, прерываем обход
            max_iterations -= 1;
            if max_iterations == 0 {
                info!("Достигнут лимит итераций для границы #{}", borders_count);
                break;
            }

            // Добавляем текущий пиксель в контур и отмечаем как обработанный
            processed.insert(current);

            // Находим соседей с высотой 0 для текущего пикселя
            let mut void_neighbors = Vec::new();
            for (dx, dz) in DIRECTIONS.iter() {
                let nx = current.x as i32 + dx;
                let nz = current.y as i32 + dz;

                // Проверяем границы
                if nx >= 0 && nx < heightmap.width() as i32 &&
                    nz >= 0 && nz < heightmap.height() as i32 {
                    let nx = nx as u32;
                    let nz = nz as u32;

                    // Если сосед пустой, добавляем как соседа границы
                    if heightmap.get_pixel(nx, nz)[0] == VOID_HEIGHT {
                        void_neighbors.push(Point { x: nx, y: nz });
                    }
                }
            }

            // Сохраняем информацию о текущем пикселе границы
            boundary.push(BorderPixel {
                position: current,
                neighbors: void_neighbors,
            });

            // Ищем следующий пиксель границы, начиная с направления,
            // перпендикулярного последнему направлению движения
            // это типичный подход алгоритма Мура: повернуть на 90° против часовой стрелки
            let next_start_direction = (prev_direction + 6) % 8; // +6 даёт поворот на 270° (т.е. -90°)

            let mut found_next = false;
            let mut next_pixel = current;
            let mut next_direction = 0;

            // Пробуем все 8 направлений, начиная с выбранного
            for i in 0..8 {
                let dir_idx = (next_start_direction + i) % 8;
                let (dx, dz) = MOORE_DIRECTIONS[dir_idx];

                let nx = current.x as i32 + dx;
                let nz = current.y as i32 + dz;

                // Проверяем, что соседний пиксель внутри чанка и границ карты высот
                if nx >= start_x as i32 && nx < end_x as i32 &&
                    nz >= start_z as i32 && nz < end_z as i32 &&
                    nx >= 0 && nx < heightmap.width() as i32 &&
                    nz >= 0 && nz < heightmap.height() as i32 {

                    let point = Point { x: nx as u32, y: nz as u32 };

                    // Проверяем, что это граничный пиксель (ненулевая высота)
                    if heightmap.get_pixel(point.x, point.y)[0] != VOID_HEIGHT {
                        // Проверяем, имеет ли пиксель хотя бы одного пустого соседа
                        let mut is_boundary = false;
                        for (ndx, ndz) in DIRECTIONS.iter() {
                            let nnx = point.x as i32 + ndx;
                            let nnz = point.y as i32 + ndz;

                            if nnx >= 0 && nnx < heightmap.width() as i32 &&
                                nnz >= 0 && nnz < heightmap.height() as i32 {
                                if heightmap.get_pixel(nnx as u32, nnz as u32)[0] == VOID_HEIGHT {
                                    is_boundary = true;
                                    break;
                                }
                            } else {
                                is_boundary = true;
                                break;
                            }
                        }

                        if is_boundary {
                            next_pixel = point;
                            next_direction = dir_idx;
                            found_next = true;
                            break;
                        }
                    }
                }
            }

            // Если не нашли следующий пиксель или достигли начальной точки, завершаем обход
            if !found_next {
                info!("Не найден следующий пиксель границы. Завершаем обход.");
                break;
            }

            // Если вернулись к начальной точке и обошли достаточно пикселей, завершаем обход
            if boundary.len() > 2 && next_pixel.x == start_point.x && next_pixel.y == start_point.y {
                info!("Контур замкнулся. Завершаем обход.");
                break;
            }

            // Переходим к следующему пикселю и сохраняем направление
            current = next_pixel;
            prev_direction = next_direction;
        }

        // Выводим информацию о текущем контуре
        info!("Граница #{} содержит {} пикселей", borders_count, boundary.len());

        // Проверяем минимальный размер контура (отфильтровываем шум)
        if boundary.len() < 3 {
            info!("Пропускаем слишком маленький контур (меньше 3 пикселей)");
            continue;
        }

        // Выводим информацию о пикселях контура
        for (i, pixel) in boundary.iter().enumerate() {
            info!("Пиксель #{}: position: ({}, {}), соседи: {}",
                  i,
                  pixel.position.x,
                  pixel.position.y,
                  pixel.neighbors.iter()
                      .map(|n| format!("({}, {})", n.x, n.y))
                      .collect::<Vec<_>>()
                      .join(", "));
        }

        // Удаляем обработанные пиксели из списка необработанных
        border_pixels.retain(|point| !processed.contains(point));
    }

    info!("Всего найдено {} границ в чанке", borders_count);
}
