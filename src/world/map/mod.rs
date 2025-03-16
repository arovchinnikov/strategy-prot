use bevy::app::{App, Startup};
use crate::world::map::terrain::spawn_terrain_chunks;

mod terrain;

pub fn build(app: &mut App) {
    app.add_systems(Startup, spawn_terrain_chunks);
}
