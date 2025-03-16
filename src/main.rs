mod scene;
mod init;
mod world;

use bevy::prelude::*;
use init::*;

fn main() {
    let mut app = App::new();
    init(&mut app);

    app.run();
}
