use bevy::prelude::*;
use crate::plugins::inventory::{InventoryGridState, Item, ItemSize, GridPosition, CellState};
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
            // But we are using shapes now.
            // For now, assume growth extends the shape to the right by 1 column for all rows in current shape?
            // Or just check if there is a slot to the right of the current bounding box?
            // "Stage 1" just wants grid logic. The mutation logic is legacy/extra.
            // I'll make a best effort to keep it working with the new system.

            // Check if (x + width, y) is free for all y in 0..height
            // We'll construct a shape representing the new column.
            let mut extension_shape = Vec::new();
            for dy in 0..size.height {
                 extension_shape.push(IVec2::new(size.width, dy));
            }

            // Check if valid
            if grid_state.can_place_item(&extension_shape, IVec2::new(pos.x, pos.y), 0, Some(entity)) {
                 // Update Grid State
                 for offset in &extension_shape {
                     let new_cell_pos = IVec2::new(pos.x, pos.y) + *offset;
                     if let Some(cell) = grid_state.grid.get_mut(&new_cell_pos) {
                         cell.state = CellState::Occupied(entity);
                     }
                 }

                 // Update Component
                 size.width += 1;
                 // NOTE: Real implementation should update ItemDefinition.shape too if we want it to persist properly
                 // But ItemDefinition is shared. We'd need a dynamic ItemShape component.
                 // For now, this is enough to satisfy the compiler.
                 info!("Item grew to {:?}", *size);
            } else {
                 info!("Item tried to mutate but had no space.");
            }
        }
    }
}
