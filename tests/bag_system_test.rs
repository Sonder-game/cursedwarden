use bevy::prelude::*;
use cursed_warden::plugins::inventory::{InventoryPlugin, InventoryGridState, Item, ItemRotation, GridPosition, CellState};
use cursed_warden::plugins::items::{ItemDatabase, ItemDefinition, ItemType, MaterialType, ItemRarity, BagType};
use cursed_warden::plugins::metagame::{PersistentInventory, SavedItem};

// Helper to create a minimal app with necessary plugins
fn create_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.init_resource::<ItemDatabase>();
    app.init_resource::<InventoryGridState>();
    app.init_resource::<PersistentInventory>();
    app
}

fn create_bag_def(id: &str, w: u8, h: u8) -> ItemDefinition {
    let mut shape = Vec::new();
    for y in 0..h {
        for x in 0..w {
            shape.push(IVec2::new(x as i32, y as i32));
        }
    }

    ItemDefinition {
        id: id.to_string(),
        name: id.to_string(),
        width: w, height: h, shape,
        material: MaterialType::Flesh,
        item_type: ItemType::Bag { bag_type: BagType::Default },
        rarity: ItemRarity::Common,
        price: 0,
        tags: vec![], synergies: vec![],
        attack: 0.0, defense: 0.0, speed: 0.0,
    }
}

#[test]
fn test_bag_mechanics() {
    let mut app = create_app();

    // Setup Item DB
    let mut item_db = app.world_mut().resource_mut::<ItemDatabase>();
    let bag_2x2 = create_bag_def("bag_2x2", 2, 2);
    let bag_1x1 = create_bag_def("bag_1x1", 1, 1);

    item_db.items.insert("bag_2x2".to_string(), bag_2x2.clone());
    item_db.items.insert("bag_1x1".to_string(), bag_1x1.clone());

    // 1. Initial State: Grid should be empty
    let grid_state_resource = app.world().resource::<InventoryGridState>();
    // Wait, InventoryGridState::default() is empty.
    assert!(grid_state_resource.grid.is_empty());
    assert!(grid_state_resource.bags.is_empty());

    // 2. Place First Bag (2x2) at (2,2)
    // First bag can be placed anywhere (adjacency rule doesn't apply to first)
    // We simulate the logic executed in `handle_drag_drop` or `load_inventory_state`.

    // Manually insert bag into grid_state
    let entity_bag1 = Entity::from_raw(1);
    let mut grid_state = app.world_mut().resource_mut::<InventoryGridState>();

    // Validate placement logic
    assert!(grid_state.can_place_bag(&bag_2x2.shape, IVec2::new(2,2), 0, None));

    // Place it
    grid_state.bags.insert(entity_bag1, (IVec2::new(2,2), 0, bag_2x2.clone()));
    grid_state.recalculate_grid();

    assert_eq!(grid_state.grid.len(), 4); // 2x2 = 4 slots
    assert!(grid_state.grid.contains_key(&IVec2::new(2,2)));
    assert!(grid_state.grid.contains_key(&IVec2::new(3,3)));

    // 3. Test Bag Overlap
    // Try to place another bag at (3,2). Overlaps (3,2) and (3,3) of the first bag.
    // can_place_bag should return false.
    assert!(!grid_state.can_place_bag(&bag_2x2.shape, IVec2::new(3,2), 0, None));

    // 4. Test Bag Adjacency
    // Try to place a bag far away at (10,10). Should fail adjacency check.
    assert!(!grid_state.can_place_bag(&bag_1x1.shape, IVec2::new(10,10), 0, None));

    // Try to place adjacent (4,2). The first bag is x:2..4 (occupies 2,3). So right edge is x=3.
    // Placing at (4,2) touches (3,2).
    assert!(grid_state.can_place_bag(&bag_1x1.shape, IVec2::new(4,2), 0, None));

    // Place it
    let entity_bag2 = Entity::from_raw(2);
    grid_state.bags.insert(entity_bag2, (IVec2::new(4,2), 0, bag_1x1.clone()));
    grid_state.recalculate_grid();

    assert_eq!(grid_state.grid.len(), 5); // 4 + 1

    // 5. Test Item Placement
    // Try to place item in valid slot (2,2)
    let item_sword_def = ItemDefinition {
        id: "sword".to_string(),
        name: "Sword".to_string(),
        width: 1, height: 2,
        shape: vec![IVec2::new(0,0), IVec2::new(0,1)],
        material: MaterialType::Steel,
        item_type: ItemType::Weapon,
        rarity: ItemRarity::Common,
        price: 0,
        tags: vec![], synergies: vec![],
        attack: 0.0, defense: 0.0, speed: 0.0,
    };

    // Valid placement: (2,2) and (2,3) are in Bag 1
    assert!(grid_state.can_place_item(&item_sword_def.shape, IVec2::new(2,2), 0, None));

    // Invalid placement: (4,2) is Bag 2 (Valid), but (4,3) is Empty (Invalid)
    assert!(!grid_state.can_place_item(&item_sword_def.shape, IVec2::new(4,2), 0, None));

    // Occupy slot
    if let Some(cell) = grid_state.grid.get_mut(&IVec2::new(2,2)) {
        cell.state = CellState::Occupied(Entity::from_raw(99));
        cell.owner_bag = Some(Entity::from_raw(1)); // Assume owned by bag 1
    }
    if let Some(cell) = grid_state.grid.get_mut(&IVec2::new(2,3)) {
        cell.state = CellState::Occupied(Entity::from_raw(99));
        cell.owner_bag = Some(Entity::from_raw(1)); // Assume owned by bag 1
    }

    // Test overlap
    assert!(!grid_state.can_place_item(&item_sword_def.shape, IVec2::new(2,2), 0, None)); // Occupied

    // Test ignore self
    assert!(grid_state.can_place_item(&item_sword_def.shape, IVec2::new(2,2), 0, Some(Entity::from_raw(99))));
}
