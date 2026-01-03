use bevy::prelude::*;
use cursed_warden::plugins::inventory::{InventoryPlugin, InventoryGridState, Item, ItemSize, GridPosition};
use cursed_warden::plugins::combat::{CombatPlugin, Health, Attack, Defense, Speed, ActionMeter, MaterialType, UnitType};
use cursed_warden::plugins::metagame::{MetagamePlugin, SaveData, PlayerStats, GlobalTime};
use cursed_warden::plugins::items::{ItemsPlugin, ItemDefinition};
use cursed_warden::plugins::core::{CorePlugin, GameState};

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

        // Test 1: Place item in empty spot
        let size = ItemSize { width: 2, height: 2 };
        let pos = IVec2::new(0, 0);
        assert!(grid_state.is_area_free(pos, size, None));
    } // Drop grid_state ref

    // Test 2: Occupy spot and check collision
    let item_entity = app.world_mut().spawn_empty().id();

    {
        let mut grid_state = app.world_mut().resource_mut::<InventoryGridState>();
        for y in 0..2 {
            for x in 0..2 {
                grid_state.cells.insert(IVec2::new(x, y), item_entity);
            }
        }

        let size = ItemSize { width: 2, height: 2 };
        let pos = IVec2::new(0, 0);

        // Try to place another item overlapping
        assert!(!grid_state.is_area_free(pos, size, Some(Entity::PLACEHOLDER))); // Different entity

        // Try to place same item (should be valid to move self)
        assert!(grid_state.is_area_free(pos, size, Some(item_entity)));

        // Try out of bounds
        assert!(!grid_state.is_area_free(IVec2::new(4, 4), size, None)); // 4+2 = 6 > 5
    }
}

#[test]
fn test_combat_simulation_loop() {
    let mut app = setup_app();

    // Setup Attacker
    let attacker = app.world_mut().spawn((
        Health { current: 100.0, max: 100.0 },
        Attack { value: 10.0 },
        Speed { value: 100.0 }, // High speed to trigger quickly
        ActionMeter { value: 950.0, threshold: 1000.0 },
        MaterialType::Steel,
        UnitType::Human,
    )).id();

    // Setup Defender
    let defender = app.world_mut().spawn((
        Health { current: 50.0, max: 50.0 },
        Defense { value: 0.0 },
        UnitType::Monster,
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
            material: cursed_warden::plugins::items::MaterialType::Steel,
            item_type: cursed_warden::plugins::items::ItemType::Weapon
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
