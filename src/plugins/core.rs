use bevy::prelude::*;

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
           .add_systems(Startup, spawn_camera);
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d::default());
}

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameState {
    AssetLoading,
    MainMenu,
    DayPhase,
    #[default]
    EveningPhase, // Inventory Management
    NightPhase,   // Auto-battle
    EventResolution,
    GameOver,
}
