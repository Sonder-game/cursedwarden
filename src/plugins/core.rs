use bevy::prelude::*;

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
           .add_sub_state::<DaySubState>()
           // FIX: Transition from AssetLoading to EveningPhase automatically
           .add_systems(Update, initial_transition.run_if(in_state(GameState::AssetLoading)));
    }
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

fn initial_transition(mut next_state: ResMut<NextState<GameState>>) {
    info!("Assets loaded, transitioning to EveningPhase");
    next_state.set(GameState::EveningPhase);
}
