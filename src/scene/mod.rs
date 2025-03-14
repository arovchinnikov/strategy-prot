use bevy::prelude::*;
use crate::scene::states::GameState;

mod loading;
mod main_menu;
mod game;
mod states;

pub fn build(app: &mut App) {
    app.add_state::<GameState>();

    loading::build(app);
    main_menu::build(app);
    game::build(app);
}