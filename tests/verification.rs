use bevy::prelude::*;
use cursed_warden::plugins::inventory::{InventoryPlugin, InventoryGridState, Item, ItemSize, GridPosition, Cell, CellState};
use cursed_warden::plugins::combat::{CombatPlugin, Health, Attack, Defense, Speed, ActionMeter, MaterialType, UnitType, Team};
use cursed_warden::plugins::metagame::{MetagamePlugin, SaveData, PlayerStats, GlobalTime};
use cursed_warden::plugins::items::{ItemsPlugin, ItemDefinition};
use cursed_warden::plugins::core::CorePlugin;

// Helper to setup app
fn setup_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
       .add_plugins(bevy::state::app::StatesPlugin)
       .add_plugins(CorePlugin)
       .add_plugins(InventoryPlugin)
       .add_plugins(CombatPlugin)
       .add_plugins(MetagamePlugin)
       .add_plugins(ItemsPlugin);
    app
}

#[test]
fn test_inventory_placement_logic() {
    let mut app = setup_app();

    // Access Grid State
    {
        let mut grid_state = app.world_mut().resource_mut::<InventoryGridState>();
        grid_state.width = 5;
        grid_state.height = 5;
        // Make all cells valid for this test (Free)
        grid_state.grid.clear();
        for y in 0..5 {
            for x in 0..5 {
                grid_state.grid.insert(IVec2::new(x, y), Cell { state: CellState::Free });
            }
        }

        // Test 1: Place item in empty spot
        // Shape for 2x2
        let shape = vec![IVec2::new(0,0), IVec2::new(1,0), IVec2::new(0,1), IVec2::new(1,1)];
        let pos = IVec2::new(0, 0);
        assert!(grid_state.can_place_item(&shape, pos, 0, None));
    } // Drop grid_state ref

    // Test 2: Occupy spot and check collision
    let item_entity = app.world_mut().spawn_empty().id();

    {
        let mut grid_state = app.world_mut().resource_mut::<InventoryGridState>();
        grid_state.width = 5;
        grid_state.height = 5;
        // Re-init grid
        grid_state.grid.clear();
         for y in 0..5 {
            for x in 0..5 {
                grid_state.grid.insert(IVec2::new(x, y), Cell { state: CellState::Free });
            }
        }

        // Occupy 2x2 at 0,0
        let shape = vec![IVec2::new(0,0), IVec2::new(1,0), IVec2::new(0,1), IVec2::new(1,1)];
        for offset in &shape {
             if let Some(cell) = grid_state.grid.get_mut(offset) {
                 cell.state = CellState::Occupied(item_entity);
             }
        }

        let pos = IVec2::new(0, 0);

        // Try to place another item overlapping
        assert!(!grid_state.can_place_item(&shape, pos, 0, Some(Entity::PLACEHOLDER))); // Different entity

        // Try to place same item (should be valid to move self)
        assert!(grid_state.can_place_item(&shape, pos, 0, Some(item_entity)));

        // Try out of bounds
        // 4,4 with 2x2 shape -> (4,4), (5,4), (4,5), (5,5). (5,x) and (x,5) are out of bounds (0..4).
        assert!(!grid_state.can_place_item(&shape, IVec2::new(4, 4), 0, None));
    }
}

#[test]
fn test_combat_simulation_loop() {
    let mut app = setup_app();

    // Setup Attacker
    let attacker = app.world_mut().spawn((
        Health { current: 100.0, max: 100.0 },
        Attack { value: 10.0 },
        Defense { value: 5.0 },
        Speed { value: 100.0 }, // High speed to trigger quickly
        ActionMeter { value: 950.0, threshold: 1000.0 },
        MaterialType::Steel,
        UnitType::Human,
        Team::Player,
    )).id();

    // Setup Defender
    let defender = app.world_mut().spawn((
        Health { current: 50.0, max: 50.0 },
        Attack { value: 5.0 },
        Defense { value: 0.0 },
        Speed { value: 10.0 },
        ActionMeter { value: 0.0, threshold: 1000.0 },
        MaterialType::Flesh,
        UnitType::Monster,
        Team::Enemy,
    )).id();

    // Run updates
    // We need to run enough updates for FixedTime to tick
    // Since we are in MinimalPlugins, we might need to manually advance time or run schedule

    // Manually run the combat system for deterministic test
    let mut schedule = Schedule::default();
    schedule.add_systems((
        cursed_warden::plugins::combat::tick_timer_system,
        cursed_warden::plugins::combat::combat_turn_system
    ).chain());

    // Tick 1: Meter goes 950 -> 1050 (Speed 100). Should attack.
    schedule.run(app.world_mut());

    let defender_health = app.world().get::<Health>(defender).unwrap();
    // Steel vs Monster = 0.8x. Attack 10 * 0.8 = 8. Defense 0.
    // GDD Formula: 2 * Raw - Defense (if Raw >= Defense)
    // 2 * 8 - 0 = 16.
    // HP: 50 - 16 = 34.
    assert_eq!(defender_health.current, 34.0);

    let attacker_meter = app.world().get::<ActionMeter>(attacker).unwrap();
    // Meter: 1050 - 1000 = 50.
    assert_eq!(attacker_meter.value, 50.0);
}

#[test]
fn test_save_data_creation() {
    let mut app = setup_app();

    // Setup State
    let mut stats = app.world_mut().resource_mut::<PlayerStats>();
    stats.thalers = 999;

    let mut time = app.world_mut().resource_mut::<GlobalTime>();
    time.hour = 12;

    // Spawn an Item
    app.world_mut().spawn((
        Item,
        GridPosition { x: 2, y: 3 },
        ItemDefinition {
            id: "test_sword".to_string(),
            name: "Test".to_string(),
            width: 1, height: 1,
            shape: vec![IVec2::new(0,0)],
            material: cursed_warden::plugins::items::MaterialType::Steel,
            item_type: cursed_warden::plugins::items::ItemType::Weapon,
            tags: vec![],
            synergies: vec![],
            attack: 10.0,
            defense: 0.0,
            speed: 0.0,
        }
    ));

    // Extract Data
    // We clone the resources first to avoid holding borrow during query creation
    let stats = app.world().resource::<PlayerStats>().clone();
    let time = app.world().resource::<GlobalTime>().clone();

    // QueryState is better.
    let mut query_state = app.world_mut().query_filtered::<(&ItemDefinition, &GridPosition), With<Item>>();

    let mut saved_items = Vec::new();
    for (def, pos) in query_state.iter(app.world()) {
        saved_items.push(cursed_warden::plugins::metagame::SavedItem {
            item_id: def.id.clone(),
            grid_x: pos.x,
            grid_y: pos.y,
            rotation: 0,
        });
    }

    let save_data = SaveData {
        player_stats: stats,
        global_time: time,
        inventory: saved_items,
    };

    // Assertions
    assert_eq!(save_data.player_stats.thalers, 999);
    assert_eq!(save_data.inventory.len(), 1);
    assert_eq!(save_data.inventory[0].grid_x, 2);
}
