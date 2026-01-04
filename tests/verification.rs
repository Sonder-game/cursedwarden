#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use cursed_warden::plugins::items::{ItemsPlugin, ItemDatabase, ItemDefinition, RecipeDefinition};
    use cursed_warden::plugins::inventory::{InventoryPlugin, InventoryGridState, LinkLine, spawn_item_entity, InventoryGridContainer, CellState, GridPosition};
    use cursed_warden::plugins::core::GameState;
    use cursed_warden::plugins::metagame::{MetagamePlugin};

    #[test]
    fn test_recipe_logic() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<GameState>();

        app.init_resource::<ButtonInput<KeyCode>>();
        app.init_resource::<ButtonInput<MouseButton>>();

        app.add_plugins(ItemsPlugin);
        app.add_plugins(MetagamePlugin);
        app.add_plugins(InventoryPlugin);

        let mut next_state = app.world_mut().resource_mut::<NextState<GameState>>();
        next_state.set(GameState::EveningPhase);

        app.update();
        app.update();

        let container = app.world_mut().query_filtered::<Entity, With<InventoryGridContainer>>().single(app.world());

        let item_db = app.world().resource::<ItemDatabase>();
        let sword_def = item_db.items.get("wooden_sword").expect("Wooden sword not found").clone();
        let stone_def = item_db.items.get("whetstone").expect("Whetstone not found").clone();

        // Use valid grid coordinates (1,2) is start of default grid
        app.add_systems(Update, move |mut commands: Commands, mut grid_state: ResMut<InventoryGridState>, q_container: Query<Entity, With<InventoryGridContainer>>| {
            if let Ok(c) = q_container.get_single() {
                // Check if already spawned to avoid loop. (1,2) is a valid spot.
                if let Some(cell) = grid_state.grid.get(&IVec2::new(1,2)) {
                    if let CellState::Free = cell.state {
                         println!("Spawning items for test...");
                         spawn_item_entity(&mut commands, c, &sword_def, IVec2::new(1,2), 0, &mut grid_state);
                         // Sword (1x2) at (1,2) occupies (1,2) and (1,3).
                         // Whetstone (1x1) needs to be neighbor.
                         // (1,4) is valid and neighbor to (1,3).
                         spawn_item_entity(&mut commands, c, &stone_def, IVec2::new(1,4), 0, &mut grid_state);
                    }
                } else {
                     println!("Coordinate (1,2) not found in grid!");
                }
            }
        });

        app.update();
        app.update();

        // Debugging
        let item_count = app.world_mut().query::<(&ItemDefinition, &GridPosition)>().iter(app.world()).count();
        println!("Items found in world: {}", item_count);
        for (def, pos) in app.world_mut().query::<(&ItemDefinition, &GridPosition)>().iter(app.world()) {
            println!("Item: {} at {:?}", def.name, pos);
        }

        let link_count = app.world_mut().query::<&LinkLine>().iter(app.world()).count();
        assert!(link_count > 0, "Should detect recipe link between Wooden Sword and Whetstone. Found {} links.", link_count);
    }
}
