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

            // Mutation: Grow in size (e.g., extend shape to the right)
            // We need to calculate the current bounding box to know what "right" means relative to the shape?
            // Or just pick a random adjacent cell?

            // Let's try to add a cell at (max_x + 1, 0) relative to item anchor, if valid.
            // Find current max x in shape
            let max_x = item.shape.iter().map(|p| p.x).max().unwrap_or(0);
            let new_offset = IVec2::new(max_x + 1, 0); // Try to grow right at top row

            // Create a temporary shape with the new cell
            let mut test_shape = vec![new_offset];

            // Check if valid placement for the NEW part
            // Note: can_place_item checks against OTHER items.
            if grid_state.can_place_item(&test_shape, pos.0, rot.0, Some(entity), false) {
                 // Update Component
                 item.shape.push(new_offset);

                 // Rebuild grid to reflect the change
                 // In the new architecture, we should rely on `InventoryChangedEvent` or manual rebuild if we modify shapes directly.
                 // Ideally we'd send an event, but here we can just rebuild lazily or expect the next frame to handle it if we triggered a change?
                 // But wait, `can_place_item` relies on `grid_state.slots`. If we modify the item shape, we need to update the grid state.
                 // The `rebuild` method in `InventoryGridState` takes queries.
                 // Since we are iterating mutably over items, we can't call rebuild easily here with queries.
                 // However, we can manually update the slot for the new cell.

                 let rotated_offset = InventoryGridState::get_rotated_shape(&vec![new_offset], rot.0);
                 if let Some(final_offset) = rotated_offset.first() {
                     let slot_pos = pos.0 + *final_offset;
                     if let Some(slot) = grid_state.slots.get_mut(&slot_pos) {
                         slot.occupier = Some(entity);
                     }
                 }

                 info!("Item grew! Added cell at {:?}", new_offset);
            } else {
                 info!("Item tried to mutate but had no space.");
            }
        }
    }
}
