use bevy::prelude::*;
use crate::plugins::inventory::{InventoryGridState, GridPosition, ItemRotation};
use crate::plugins::items::{ItemDatabase, ItemDefinition};
use crate::plugins::core::GameState;

pub struct VisualizationPlugin;

impl Plugin for VisualizationPlugin {
    fn build(&self, app: &mut App) {
        // Disabled visualization systems until they are updated for the new inventory
        // app.add_systems(Update, (draw_synergy_lines, draw_recipe_lines).run_if(in_state(GameState::EveningPhase)));
    }
}

// -------------------------------------------------------------------------------------------------
// Visualization Systems - STUBBED
// -------------------------------------------------------------------------------------------------

// TODO: Re-implement visualizers for new InventoryGridState (based on slots hashmap, not direct item queries)
// Code removed for clarity during refactor.
