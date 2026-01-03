use bevy::prelude::*;
use crate::plugins::inventory::{InventoryGridState, Item, ItemSize, GridPosition};
use rand::Rng;

pub fn mutation_system(
    mut q_items: Query<(Entity, &mut ItemSize, &GridPosition), With<Item>>,
    mut grid_state: ResMut<InventoryGridState>,
    // In a real implementation, we'd check infection level here
    // infection: Res<GlobalInfection>,
) {
    // This system should run ONCE per Evening->Night transition.
    // For now, we'll assume it's called by a schedule or state change trigger.

    let mut rng = rand::thread_rng();

    // GDD: P_mut = Base + Infection * 0.5. Let's assume 10% base chance for verification.
    let mutation_chance = 0.10;

    for (entity, mut size, pos) in q_items.iter_mut() {
        if rng.gen_bool(mutation_chance) {
            info!("Item {:?} is mutating!", entity);

            // Mutation: Grow in size (e.g., width + 1)
            // We need to check if the new size fits.

            // Check if valid
            // We temporarily remove the current item from grid to check self-overlap (not needed for growth, but good practice)
            // But here we are growing to the right.
            // We need to check if (x + width, y) is free.

            let grow_space_free = (0..size.height).all(|dy| {
                let check_pos = IVec2::new(pos.x + size.width, pos.y + dy); // The column to the right
                grid_state.is_area_free(check_pos, ItemSize { width: 1, height: 1 }, Some(entity))
            });

            if grow_space_free {
                 // Update Grid State
                 for dy in 0..size.height {
                     let new_cell_pos = IVec2::new(pos.x + size.width, pos.y + dy);
                     grid_state.cells.insert(new_cell_pos, entity);
                 }

                 // Update Component
                 size.width += 1;
                 info!("Item grew to {:?}", *size);
            } else {
                 info!("Item tried to mutate but had no space.");
                 // GDD says: "Consume neighbor".
                 // Simplified: If blocked, maybe just change material?
            }
        }
    }
}
