use bevy::prelude::*;

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>();
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

// Sub-states or components can be added later as needed.
// For now, keeping GameState simple to avoid complexity with enum variants in States.
