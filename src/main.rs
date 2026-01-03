use bevy::prelude::*;

pub mod core;
pub mod inventory;
pub mod ui;
pub mod combat;
pub mod metagame;
pub mod narrative;

use core::{CorePlugin, GameState};
use inventory::GridInventoryPlugin;
use ui::UiPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((CorePlugin, GridInventoryPlugin, UiPlugin))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, mut state: ResMut<NextState<GameState>>) {
    commands.spawn(Camera2d::default());
    println!("Cursed Warden is starting...");

    // Jump straight to EveningPhase to show Inventory UI
    state.set(GameState::EveningPhase);
}
