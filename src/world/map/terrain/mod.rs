use bevy::prelude::*;
use bevy::render::view::RenderLayers;

pub fn spawn_terrain_chunks(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let width = 8192;
    let height = 4096;
    let chunk_size = 256;

    let num_chunks_x = width / chunk_size;
    let num_chunks_z = height / chunk_size;

    let parent_entity = commands.spawn((
        Transform::default(),
        GlobalTransform::default(),
        Visibility::default()
    )).id();

    let mut chunk_num_id = 0;

    for z in 0..num_chunks_z {
        for x in 0..num_chunks_x {
            let start_x = x * chunk_size;
            let start_z = z * chunk_size;

            let material_handle = materials.add(StandardMaterial {
                base_color: Color::srgb(0.3, 0.5, 0.4),
                perceptual_roughness: 1.0,
                ..default()
            });

            let terrain_chunk = commands.spawn((
                Mesh3d::from(Handle::default()),
                MeshMaterial3d::from(material_handle),
                Transform {
                    translation: Vec3::new(start_x as f32, 0.0, start_z as f32),
                    scale: Vec3::new(1.0, 1.0, 1.0),
                    ..default()
                },
                RenderLayers::layer(1)
            )).id();

            commands.entity(parent_entity).insert_children(chunk_num_id as usize, &[terrain_chunk]);

            chunk_num_id += 1;
        }
    }
}
