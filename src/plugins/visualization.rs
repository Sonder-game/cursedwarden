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
    _grid_state: Res<InventoryGridState>, // Unused in this function if we only iterate items?
                                          // logic uses grid_state.grid to check occupancy
) {
    // Iterate items to find active synergies
    for (entity, pos, rot, def) in q_items.iter() {
        if def.synergies.is_empty() { continue; }

        let start_node_transform = if let Ok(t) = q_transforms.get(entity) {
            t
        } else {
            continue;
        };

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
/// Gold: Ready (all ingredients present and connected).
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

    for recipe in &item_db.recipes {
        if recipe.ingredients.len() < 2 { continue; }

        // Naive Check:
        // If recipe has exactly 2 ingredients (most common case for Backpack Battles basic recipes),
        // we check if we have a pair that matches.

        if recipe.ingredients.len() == 2 {
            let id_a = &recipe.ingredients[0];
            let id_b = &recipe.ingredients[1];

            // Find candidates
            let items_a: Vec<_> = items_on_grid.iter().filter(|(_, def, _, _)| def.id == *id_a).collect();
            let items_b: Vec<_> = items_on_grid.iter().filter(|(_, def, _, _)| def.id == *id_b).collect();

            for (entity_a, def_a, pos_a, rot_a) in &items_a {
                for (entity_b, def_b, pos_b, rot_b) in &items_b {
                    // If IDs are same, ensure entities are different
                    if entity_a == entity_b { continue; }

                    if are_adjacent(pos_a, rot_a, def_a, pos_b, rot_b, def_b) {
                        // Found a matching pair!
                        // Since it's a 2-ingredient recipe, it is READY.
                        // Draw GOLD line.
                         if let (Ok(t_a), Ok(t_b)) = (q_transforms.get(*entity_a), q_transforms.get(*entity_b)) {
                             let p1 = t_a.translation().truncate();
                             let p2 = t_b.translation().truncate();

                             gizmos.line_2d(p1, p2, Color::srgb(1.0, 0.84, 0.0)); // Gold
                         }
                    }
                }
            }
        } else {
            // For > 2 ingredients, just check pairs and draw Blue (Potential).
             for i in 0..recipe.ingredients.len() {
                for j in (i+1)..recipe.ingredients.len() {
                    let id_a = &recipe.ingredients[i];
                    let id_b = &recipe.ingredients[j];

                    let items_a: Vec<_> = items_on_grid.iter().filter(|(_, def, _, _)| def.id == *id_a).collect();
                    let items_b: Vec<_> = items_on_grid.iter().filter(|(_, def, _, _)| def.id == *id_b).collect();

                    for (entity_a, def_a, pos_a, rot_a) in &items_a {
                        for (entity_b, def_b, pos_b, rot_b) in &items_b {
                            if entity_a == entity_b { continue; }

                            if are_adjacent(pos_a, rot_a, def_a, pos_b, rot_b, def_b) {
                                 // Draw Blue line for partial connection
                                 if let (Ok(t_a), Ok(t_b)) = (q_transforms.get(*entity_a), q_transforms.get(*entity_b)) {
                                     let p1 = t_a.translation().truncate();
                                     let p2 = t_b.translation().truncate();
                                     gizmos.line_2d(p1, p2, Color::srgb(0.0, 0.0, 1.0)); // Blue
                                 }
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
    // Adjacent means sharing an edge (Manhattan distance == 1)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::items::{ItemType, MaterialType, ItemRarity};

    fn create_dummy_item(w: u8, h: u8) -> ItemDefinition {
        let mut shape = Vec::new();
        for y in 0..h {
            for x in 0..w {
                shape.push(IVec2::new(x as i32, y as i32));
            }
        }
        ItemDefinition {
            id: "test".to_string(), name: "Test".to_string(),
            width: w, height: h, shape,
            material: MaterialType::Steel, item_type: ItemType::Weapon,
            rarity: ItemRarity::Common, price: 0,
            tags: vec![], synergies: vec![],
            attack: 0.0, defense: 0.0, speed: 0.0,
        }
    }

    #[test]
    fn test_adjacency() {
        let item_1x1 = create_dummy_item(1, 1);

        // Item A at (0,0)
        let pos_a = GridPosition { x: 0, y: 0 };
        let rot_a = ItemRotation { value: 0 };

        // Item B at (1,0) -> Adjacent Right
        let pos_b = GridPosition { x: 1, y: 0 };
        let rot_b = ItemRotation { value: 0 };

        assert!(are_adjacent(&pos_a, &rot_a, &item_1x1, &pos_b, &rot_b, &item_1x1));

        // Item C at (2,0) -> Not Adjacent (Gap)
        let pos_c = GridPosition { x: 2, y: 0 };
        assert!(!are_adjacent(&pos_a, &rot_a, &item_1x1, &pos_c, &rot_b, &item_1x1));

        // Item D at (0,1) -> Adjacent Down
        let pos_d = GridPosition { x: 0, y: 1 };
        assert!(are_adjacent(&pos_a, &rot_a, &item_1x1, &pos_d, &rot_b, &item_1x1));
    }
}
