use bevy::prelude::*;

mod plugins;
use plugins::core::{CorePlugin, GameState};
use plugins::inventory::InventoryPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(CorePlugin)
        .add_plugins(InventoryPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d::default());
    println!("Cursed Warden is starting...");
}
