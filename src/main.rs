use bevy::prelude::*;

use cursed_warden::plugins::combat::CombatPlugin;
use cursed_warden::plugins::core::CorePlugin;
use cursed_warden::plugins::inventory::InventoryPlugin;
use cursed_warden::plugins::items::ItemsPlugin;
use cursed_warden::plugins::metagame::MetagamePlugin;

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
