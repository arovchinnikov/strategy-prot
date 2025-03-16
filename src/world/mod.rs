use bevy::app::App;

mod map;
mod camera;

pub fn build(app: &mut App) {
    map::build(app);
    camera::build(app);
}
