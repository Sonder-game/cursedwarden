use bevy::prelude::*;
use crate::plugins::inventory::{InventoryGridState, InventoryItem, GridPosition, ItemRotation, rotate_shape};
use rand::Rng;

pub struct MutationPlugin;

impl Plugin for MutationPlugin {
    fn build(&self, app: &mut App) {
         app.add_systems(OnEnter(crate::plugins::core::GameState::NightPhase), mutation_system);
    }
}

pub fn mutation_system(
    mut q_items: Query<(Entity, &mut InventoryItem, &GridPosition, &ItemRotation)>,
    mut grid_state: ResMut<InventoryGridState>,
    // In a real implementation, we'd check infection level here
    // infection: Res<GlobalInfection>,
) {
    let mut rng = rand::thread_rng();

    // GDD: P_mut = Base + Infection * 0.5. Let's assume 10% base chance for verification.
    let mutation_chance = 0.10;

    for (entity, mut item, pos, rot) in q_items.iter_mut() {
        if rng.gen_bool(mutation_chance) {
            info!("Item {:?} is mutating!", entity);

            // Mutation: Grow in size (e.g., width + 1)
            // 1. Calculate bounding box of current shape
            let mut max_x = 0;
            for p in &item.base_shape {
                if p.x > max_x { max_x = p.x; }
            }
            // 2. Try to add a column at x = max_x + 1
            let mut extension_shape = Vec::new();
            // Find all unique Ys at max_x
            let ys: Vec<i32> = item.base_shape.iter().filter(|p| p.x == max_x).map(|p| p.y).collect();

            for y in ys {
                extension_shape.push(IVec2::new(max_x + 1, y));
            }

            if extension_shape.is_empty() { continue; }

            // Check if valid
            // We pass the extension shape to check if THOSE specific cells are free
            // Relative to item pos.
            // Note: can_place_item will rotate the shape we pass it by rot.0
            if grid_state.can_place_item(&extension_shape, pos.0, rot.0, Some(entity)) {

                 // Update Grid State Manually
                 // We need to calculate absolute positions to update the occupancy map
                 let rotated_extension = rotate_shape(&extension_shape, rot.0);

                 for offset in rotated_extension {
                     let new_cell_pos = pos.0 + offset;
                     // In the new system, slots are just Entity IDs of bags, and occupancy is separate.
                     // We just update occupancy.
                     grid_state.occupancy.insert(new_cell_pos, entity);
                 }

                 // Update Component
                 item.base_shape.extend(extension_shape);

                 info!("Item mutated (grew)!");
            } else {
                 info!("Item tried to mutate but had no space.");
            }
        }
    }
}
