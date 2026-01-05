use bevy::prelude::*;
// use crate::plugins::inventory::{InventoryGridState, Item, ItemSize, GridPosition, CellState};
// use rand::Rng;

pub fn mutation_system(
    // mut q_items: Query<(Entity, &mut ItemSize, &GridPosition), With<Item>>,
    // mut grid_state: ResMut<InventoryGridState>,
) {
    warn!("Mutation system disabled during inventory refactor");
    /*
    // This system should run ONCE per Evening->Night transition.
    let mut rng = rand::thread_rng();
    let mutation_chance = 0.10;

    for (entity, mut size, pos) in q_items.iter_mut() {
        if rng.gen_bool(mutation_chance) {
            info!("Item {:?} is mutating!", entity);

            let mut extension_shape = Vec::new();
            for dy in 0..size.height {
                 extension_shape.push(IVec2::new(size.width, dy));
            }

            if grid_state.can_place_item(&extension_shape, IVec2::new(pos.x, pos.y), 0, Some(entity)) {
                 for offset in &extension_shape {
                     let new_cell_pos = IVec2::new(pos.x, pos.y) + *offset;
                     if let Some(cell) = grid_state.grid.get_mut(&new_cell_pos) {
                         cell.state = CellState::Occupied(entity);
                     }
                 }
                 size.width += 1;
                 info!("Item grew to {:?}", *size);
            } else {
                 info!("Item tried to mutate but had no space.");
            }
        }
    }
    */
}
