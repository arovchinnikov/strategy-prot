use bevy::prelude::*;
use bevy_framepace::FramepacePlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use crate::{scene, world};

pub fn init(app: &mut App) {
    add_bevy_plugins(app);
    add_lib_plugins(app);

    build_modules(app);
}

fn build_modules(app: &mut App) {
    scene::build(app);
    world::build(app);
}

fn add_bevy_plugins(app: &mut App) {
    let plugins = DefaultPlugins.set(
        AssetPlugin {
            file_path: "common".to_string(),
            ..default()
        }
    ).set(
        WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Game".into(),
                resolution: (800.0, 600.0).into(),
                ..default()
            }),
            ..default()
        }
    );

    app.add_plugins(plugins);
}

fn add_lib_plugins(app: &mut App) {
    let plugins = (
        WorldInspectorPlugin::new(),
        FramepacePlugin
    );

    app.add_plugins(plugins);
}
