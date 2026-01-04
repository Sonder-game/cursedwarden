use bevy::prelude::*;
use bevy::state::prelude::*;

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
           .add_sub_state::<DaySubState>()
           .add_systems(OnEnter(GameState::AssetLoading), finish_loading);
    }
}

fn finish_loading(mut next_state: ResMut<NextState<GameState>>) {
    info!("Assets loaded (mock). Transitioning to DayPhase.");
    next_state.set(GameState::DayPhase);
}

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum GameState {
   #[default]
   AssetLoading,
   #[allow(dead_code)]
   MainMenu,
   DayPhase,
   EveningPhase,          // Inventory management
   NightPhase,            // Auto-battle
   #[allow(dead_code)]
   EventResolution,       // Dialogs
   #[allow(dead_code)]
   GameOver,
}

#[derive(SubStates, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
#[source(GameState = GameState::DayPhase)]
pub enum DaySubState {
   #[default]
   Idle,
   #[allow(dead_code)]
   Trading,
   #[allow(dead_code)]
   MapTravel,
}
