use bevy::prelude::*;
use crate::plugins::core::CorePlugin;
use crate::plugins::grid::GridInventoryPlugin;

mod plugins;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(CorePlugin)
        .add_plugins(GridInventoryPlugin)
        .run();
}
