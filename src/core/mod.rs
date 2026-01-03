use bevy::prelude::*;

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>();
    }
}

#[derive(States, Debug, Clone, Copy, Default, Eq, PartialEq, Hash)]
pub enum GameState {
    #[default]
    AssetLoading,
    MainMenu,
    DayPhase,
    EveningPhase,
    NightPhase,
    EventResolution,
    GameOver,
}

#[derive(States, Debug, Clone, Copy, Default, Eq, PartialEq, Hash)]
pub enum DaySubState {
    #[default]
    Idle,
    Trading,
    MapTravel,
}
