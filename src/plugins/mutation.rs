use bevy::prelude::*;
use crate::plugins::inventory::{InventoryGridState, InventoryItem, GridPosition, ItemRotation};
use rand::Rng;

pub fn mutation_system(
    mut q_items: Query<(Entity, &mut InventoryItem, &GridPosition, &ItemRotation)>,
    mut grid_state: ResMut<InventoryGridState>,
    // In a real implementation, we'd check infection level here
    // infection: Res<GlobalInfection>,
) {
    // This system should run ONCE per Evening->Night transition.
    // For now, we'll assume it's called by a schedule or state change trigger.

    let mut rng = rand::thread_rng();

    // GDD: P_mut = Base + Infection * 0.5. Let's assume 10% base chance for verification.
    let mutation_chance = 0.10;

    for (entity, mut item, pos, rot) in q_items.iter_mut() {
        if rng.gen_bool(mutation_chance) {
            info!("Item {:?} is mutating!", entity);

            // Mutation: Grow in size.
            // With arbitrary shapes, "growth" is adding a block adjacent to the current shape.
            // Let's try to add a block to the right of the rightmost block in the shape.

            // Find rightmost block (max x)
            if let Some(rightmost) = item.shape.iter().max_by_key(|p| p.x) {
                let candidate_offset = *rightmost + IVec2::new(1, 0);

                // Construct a temporary single-block shape for validation
                let extension_shape = vec![candidate_offset];

                // Check if valid using the grid logic
                // Note: We check with the item's current rotation and position
                if grid_state.can_place_item(&extension_shape, pos.0, rot.0, Some(entity), false) {
                     // Apply mutation
                     item.shape.push(candidate_offset);

                     // We must trigger a rebuild of the grid to reflect this change
                     // However, we are iterating.
                     // Ideally we should emit an event or batch these.
                     // For now, we manually update the slot if we can find it,
                     // but grid_state.rebuild() is the safest way to ensure consistency.
                     // Since we can't easily call rebuild inside the query loop (borrow checker),
                     // we relies on the fact that inventory systems rebuild often, or we update the slot directly.

                     // Get absolute position of the new block
                     let rotated_offset = InventoryGridState::get_rotated_shape(&extension_shape, rot.0);
                     // wait, get_rotated_shape returns a vector.

                     for offset in rotated_offset {
                        let cell_pos = pos.0 + offset;
                        if let Some(slot) = grid_state.slots.get_mut(&cell_pos) {
                            slot.occupier = Some(entity);
                        }
                     }

                     info!("Item mutated! New shape len: {}", item.shape.len());
                } else {
                     info!("Item tried to mutate but had no space.");
                }
            }
        }
    }
}
