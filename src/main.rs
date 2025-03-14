mod scene;

use bevy::prelude::*;
use bevy_framepace::{FramepacePlugin, FramepaceSettings, Limiter};
use bevy_inspector_egui::quick::WorldInspectorPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Game".into(),
                resolution: (800.0, 600.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(WorldInspectorPlugin::new())
        .add_plugins(FramepacePlugin)
        .call(scene::build)
        .add_systems(Startup, setup_frame_limit)
        .run();
}

fn setup_frame_limit(mut settings: ResMut<FramepaceSettings>) {
    settings.limiter = Limiter::from_framerate(60.0);
}
