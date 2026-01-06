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

            // Mutation: Grow in size (e.g., width + 1)
            // We need to check if the new size fits.
            // With arbitrary shapes, "growing" is ambiguous.
            // Let's assume we try to add a block to the right of the bounding box.

            // 1. Calculate bounding box of current shape
            let mut max_x = 0;
            for p in &item.shape {
                if p.x > max_x { max_x = p.x; }
            }
            // 2. Try to add a column at x = max_x + 1
            let mut extension_shape = Vec::new();
            // Find all unique Ys at max_x
            let ys: Vec<i32> = item.shape.iter().filter(|p| p.x == max_x).map(|p| p.y).collect();

            for y in ys {
                extension_shape.push(IVec2::new(max_x + 1, y));
            }

            if extension_shape.is_empty() { continue; }

            // Check if valid
            // can_place_item expects the FULL shape relative to pos.
            // We want to check if the *extension* fits.
            // We can cheat by passing extension_shape as the shape.
            if grid_state.can_place_item(&extension_shape, pos.0, rot.0, Some(entity), false) {
                 // Update Grid State
                 // Note: inventory plugin rebuilds grid every frame or on change.
                 // So we just need to update the component `item.shape`.
                 // grid_state.rebuild() will be called by the system loop eventually or we can trigger it.
                 // The inventory system listens to `InventoryChangedEvent`, but here we modify the component directly.
                 // The `update_grid_visuals` might catch it if we change GridPosition, but we are changing Shape.
                 // We should ideally trigger an event or just rely on the next frame's rebuild if it runs every frame?
                 // The provided inventory.rs has `update_grid_visuals` which only updates position.
                 // And `rebuild` is called in `on_drag_end`.
                 // Wait, `rebuild` is NOT called every frame in the provided code! It's manual.
                 // Let's check inventory.rs again.
                 // It says: `grid_state.rebuild(&q_bags, &q_items);` inside `on_drag_end`.
                 // It does NOT run in Update.
                 // So we must manually update the slots here or trigger a rebuild.
                 // Since we don't have easy access to q_bags here to call rebuild, we will just update slots manually.

                 let rotated_extension = InventoryGridState::get_rotated_shape(&extension_shape, rot.0);

                 for offset in rotated_extension {
                     let new_cell_pos = pos.0 + offset;
                     if let Some(slot) = grid_state.slots.get_mut(&new_cell_pos) {
                         slot.occupier = Some(entity);
                     }
                 }

                 // Update Component
                 item.shape.extend(extension_shape);

                 info!("Item mutated (grew)!");
            } else {
                 info!("Item tried to mutate but had no space.");
            }
        }
    }
}
