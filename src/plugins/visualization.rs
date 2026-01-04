use bevy::prelude::*;
use crate::plugins::inventory::{InventoryGridState, GridPosition, ItemRotation, InventoryGridContainer};
use crate::plugins::items::{ItemDatabase, ItemDefinition};
use crate::plugins::core::GameState;

pub struct VisualizationPlugin;

impl Plugin for VisualizationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (draw_synergy_lines, draw_recipe_lines).run_if(in_state(GameState::EveningPhase)));
    }
}

// -------------------------------------------------------------------------------------------------
// Visualization Systems
// -------------------------------------------------------------------------------------------------

/// Draws lines between items that have active synergies.
/// Green lines for synergies.
fn draw_synergy_lines(
    mut gizmos: Gizmos,
    q_items: Query<(Entity, &GridPosition, &ItemRotation, &ItemDefinition)>,
    grid_state: Res<InventoryGridState>,
    q_tags: Query<&ItemDefinition>,
    q_transforms: Query<&GlobalTransform>,
) {
    // Iterate items to find active synergies
    for (entity, pos, rot, def) in q_items.iter() {
        if def.synergies.is_empty() { continue; }

        let start_node_transform = if let Ok(t) = q_transforms.get(entity) {
            t
        } else {
            continue;
        };

        // Z-Index hack: Draw slightly above 0 to be visible over UI if UI is at 0?
        // UI is typically rendered in a separate pass without depth testing against world.
        // But Gizmos are world space.
        // If we want Gizmos to show, we just draw them.
        let start_pos = start_node_transform.translation().truncate();

        for synergy in &def.synergies {
            // Calculate target grid position
            let rotated_offset_vec = InventoryGridState::get_rotated_shape(&vec![synergy.offset], rot.value);
            if rotated_offset_vec.is_empty() { continue; }
            let rotated_offset = rotated_offset_vec[0];
            let target_pos = IVec2::new(pos.x, pos.y) + rotated_offset;

            // Check if occupied
            if let Some(cell) = grid_state.grid.get(&target_pos) {
                if let crate::plugins::inventory::CellState::Occupied(target_entity) = cell.state {
                    // Check tags
                    if let Ok(target_def) = q_tags.get(target_entity) {
                        if synergy.target_tags.iter().any(|req| target_def.tags.contains(req)) {
                             // Match found! Draw line.
                             if let Ok(target_transform) = q_transforms.get(target_entity) {
                                 let end_pos = target_transform.translation().truncate();

                                 // Draw Green Line for Synergy
                                 gizmos.line_2d(start_pos, end_pos, Color::srgb(0.0, 1.0, 0.0));
                             }
                        }
                    }
                }
            }
        }
    }
}

/// Draws lines for potential recipes.
/// Blue: Potential (neighboring ingredient).
/// Gold: Ready (all ingredients present).
fn draw_recipe_lines(
    mut gizmos: Gizmos,
    q_items: Query<(Entity, &GridPosition, &ItemDefinition, &ItemRotation)>,
    item_db: Res<ItemDatabase>,
    q_transforms: Query<&GlobalTransform>,
) {
    if item_db.recipes.is_empty() { return; }

    // Collect all items on grid
    let mut items_on_grid: Vec<(Entity, &ItemDefinition, &GridPosition, &ItemRotation)> = Vec::new();
    for (e, pos, def, rot) in q_items.iter() {
        items_on_grid.push((e, def, pos, rot));
    }

    // Naive O(R * N^2) check. R=recipes, N=items. N is small (~20).
    for recipe in &item_db.recipes {
        if recipe.ingredients.len() < 2 { continue; }

        // Find which ingredients we have
        // A recipe matches if we have distinct items corresponding to the ingredients list
        // AND they are adjacent.

        // This is a graph matching problem technically.
        // Let's simplify: Check for any PAIR of ingredients that are adjacent.
        // If found, draw Blue line.
        // If ALL ingredients are found and connected, draw Gold line.

        // For simplicity in this step, let's just look for pairs.
        // If ingredient A and ingredient B are adjacent, draw line.

        for i in 0..recipe.ingredients.len() {
            for j in (i+1)..recipe.ingredients.len() {
                let id_a = &recipe.ingredients[i];
                let id_b = &recipe.ingredients[j];

                // Find items matching id_a
                let items_a: Vec<_> = items_on_grid.iter().filter(|(_, def, _, _)| def.id == *id_a).collect();
                // Find items matching id_b
                let items_b: Vec<_> = items_on_grid.iter().filter(|(_, def, _, _)| def.id == *id_b).collect();

                for (entity_a, def_a, pos_a, rot_a) in &items_a {
                    for (entity_b, def_b, pos_b, rot_b) in &items_b {
                        if entity_a == entity_b { continue; } // Should not happen if indices distinct, but good to check

                        // Check adjacency
                        if are_adjacent(pos_a, rot_a, def_a, pos_b, rot_b, def_b) {
                             // Draw line
                             if let (Ok(t_a), Ok(t_b)) = (q_transforms.get(*entity_a), q_transforms.get(*entity_b)) {
                                 let p1 = t_a.translation().truncate();
                                 let p2 = t_b.translation().truncate();

                                 // Determine color
                                 // Real logic would check if WHOLE recipe is ready.
                                 // For now, Blue for link.
                                 let color = Color::srgb(0.0, 0.0, 1.0); // Blue

                                 gizmos.line_2d(p1, p2, color);
                             }
                        }
                    }
                }
            }
        }
    }
}

fn are_adjacent(
    pos_a: &GridPosition, rot_a: &ItemRotation, def_a: &ItemDefinition,
    pos_b: &GridPosition, rot_b: &ItemRotation, def_b: &ItemDefinition
) -> bool {
    // Get all cells for A
    let shape_a = InventoryGridState::get_rotated_shape(&def_a.shape, rot_a.value);
    let cells_a: Vec<IVec2> = shape_a.iter().map(|offset| IVec2::new(pos_a.x, pos_a.y) + *offset).collect();

    // Get all cells for B
    let shape_b = InventoryGridState::get_rotated_shape(&def_b.shape, rot_b.value);
    let cells_b: Vec<IVec2> = shape_b.iter().map(|offset| IVec2::new(pos_b.x, pos_b.y) + *offset).collect();

    // Check if any cell in A is adjacent (dist 1) to any cell in B
    for ca in &cells_a {
        for cb in &cells_b {
            let dx = (ca.x - cb.x).abs();
            let dy = (ca.y - cb.y).abs();
            if dx + dy == 1 {
                return true;
            }
        }
    }
    false
}
