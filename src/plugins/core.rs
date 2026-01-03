use bevy::prelude::*;

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
           .add_sub_state::<DaySubState>();
    }
}

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum GameState {
   #[default]
   AssetLoading,
   MainMenu,
   DayPhase,
   EveningPhase,          // Inventory management
   NightPhase,            // Auto-battle
   EventResolution,       // Dialogs
   GameOver,
}

#[derive(SubStates, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
#[source(GameState = GameState::DayPhase)]
pub enum DaySubState {
   #[default]
   Idle,
   Trading,
   MapTravel,
}
