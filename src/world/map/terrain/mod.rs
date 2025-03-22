mod terrain_generator;
mod border_generator;

use std::f32::consts::PI;
use std::path::Path;
use bevy::color::palettes::basic::WHITE;
use bevy::pbr::wireframe::Wireframe;
use bevy::prelude::*;
use bevy::render::view::RenderLayers;
use image::{GrayImage, ImageReader};
use crate::world::map::terrain::terrain_generator::generate_terrain_mesh;

pub fn spawn_terrain_chunks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let width = 8192;
    let height = 4096;
    let chunk_size = 128;

    let num_chunks_x = width / chunk_size;
    let num_chunks_z = height / chunk_size;

    let heightmap = load_heightmap("common/map/heightmap.png");

    let parent_entity = commands.spawn((
        Transform::default(),
        GlobalTransform::default(),
        Visibility::default()
    )).id();

    let mut chunk_num_id = 0;

    commands.spawn((
        DirectionalLight {
            color: WHITE.into(),
            illuminance: 4500.,
            shadows_enabled: true,
            shadow_depth_bias: 0.002,
            ..default()
        },
        Transform::from_xyz(0.0, 2000.0, 0.0).with_rotation(Quat::from_axis_angle(Vec3::ONE, -PI / 6.))
    ));

    for z in 0..num_chunks_z {
        for x in 0..num_chunks_x {
            let start_x = x * chunk_size;
            let start_z = z * chunk_size;

            let material_handle = materials.add(StandardMaterial {
                base_color: Color::srgb(0.3, 0.5, 0.4),
                perceptual_roughness: 1.0,
                ..default()
            });

            let mesh = generate_terrain_mesh(start_x, start_z, &heightmap);
            if mesh.is_none() {
                continue;
            }

            let terrain_chunk = commands.spawn((
                Mesh3d::from(meshes.add(mesh.unwrap())),
                MeshMaterial3d::from(material_handle),
                Transform {
                    translation: Vec3::new(start_x as f32, 0.0, start_z as f32),
                    scale: Vec3::new(1.0, 1.0, 1.0),
                    ..default()
                },
                Wireframe,
                RenderLayers::from_layers(&[0, 1])
            )).id();

            commands.entity(parent_entity).insert_children(chunk_num_id as usize, &[terrain_chunk]);

            chunk_num_id += 1;
        }
    }
}

fn load_heightmap(path: &str) -> GrayImage {
    let img = ImageReader::open(Path::new(path))
        .expect("Failed to open image")
        .decode()
        .expect("Failed to decode image");

    img.into_luma8()
}
