use bevy::prelude::*;

mod plugins;
#[cfg(test)]
mod tests;

use plugins::combat::CombatPlugin;
use plugins::core::CorePlugin;
use plugins::inventory::InventoryPlugin;
use plugins::items::ItemsPlugin;
use plugins::metagame::MetagamePlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(CorePlugin)
        .add_plugins(InventoryPlugin)
        .add_plugins(ItemsPlugin)
        .add_plugins(CombatPlugin)
        .add_plugins(MetagamePlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d::default());
    println!("Cursed Warden is starting...");
}
