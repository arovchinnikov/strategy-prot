use bevy::app::App;

mod map;

pub fn build(app: &mut App) {
    map::build(app);
}
