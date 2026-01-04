use bevy::prelude::*;
use cursed_warden::plugins::inventory::{InventoryPlugin, InventoryGridState, Item, ItemSize, GridPosition};
use cursed_warden::plugins::combat::{CombatPlugin, MaterialType as CombatMaterialType};
use cursed_warden::plugins::metagame::{MetagamePlugin, PendingItems, PlayerStats, GlobalTime};
use cursed_warden::plugins::items::{ItemsPlugin, ItemDefinition, ItemDatabase, ItemType, MaterialType as ItemMaterialType};
use cursed_warden::plugins::core::{CorePlugin, GameState, DaySubState};

#[test]
fn test_item_transfer_from_metagame_to_inventory() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
       .add_plugins(bevy::state::app::StatesPlugin)
       .add_plugins(CorePlugin)
       .add_plugins(InventoryPlugin)
       .add_plugins(CombatPlugin)
       .add_plugins(MetagamePlugin)
       .add_plugins(ItemsPlugin);

    // Add missing resources for systems that expect input
    app.init_resource::<ButtonInput<KeyCode>>();

    // Initial update to run Startup systems (populate ItemDatabase)
    app.update();

    // 1. Setup: Add "steel_sword" to PendingItems (simulating Metagame find)
    app.world_mut().resource_mut::<PendingItems>().0.push("steel_sword".to_string());

    // 2. Ensure Database has steel_sword (ItemsPlugin loads it, but we can verify)
    {
        let db = app.world().resource::<ItemDatabase>();
        assert!(db.items.contains_key("steel_sword"), "ItemDatabase should contain steel_sword");
    }

    // 3. Transition to EveningPhase
    // This triggers: spawn_inventory_ui, apply_deferred, consume_pending_items
    app.insert_state(GameState::DayPhase);

    let mut next_state = app.world_mut().resource_mut::<NextState<GameState>>();
    next_state.set(GameState::EveningPhase);

    // 4. Run Schedule to process transition
    // We need to run enough updates. OnEnter runs on the first update after state change.
    app.update();
    app.update(); // Just in case of frame delays (though State transitions usually happen in StateTransition schedule)

    // 5. Verify Inventory Grid State
    let grid_state = app.world().resource::<InventoryGridState>();
    assert!(!grid_state.cells.is_empty(), "Grid should contain items after transition");

    // Check if the item is "Steel Sword"
    let entity = *grid_state.cells.values().next().unwrap();
    let name = app.world().get::<ItemDefinition>(entity).map(|d| d.name.as_str());
    assert_eq!(name, Some("Steel Sword"));

    // 6. Verify PendingItems is empty
    let pending = app.world().resource::<PendingItems>();
    assert!(pending.0.is_empty(), "PendingItems should be consumed");
}

#[test]
fn test_inventory_ui_persistence() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
       .add_plugins(bevy::state::app::StatesPlugin)
       .add_plugins(CorePlugin)
       .add_plugins(InventoryPlugin)
       .add_plugins(MetagamePlugin)
       .add_plugins(ItemsPlugin);

    // Start in DayPhase
    app.insert_state(GameState::DayPhase);
    app.update();

    // Transition to EveningPhase
    app.world_mut().resource_mut::<NextState<GameState>>().set(GameState::EveningPhase);
    app.update();

    // Check UI exists
    // We need to query for InventoryUiRoot. Since it's not public, we can't query it by type directly in test unless we make it pub or find it by other means.
    // But we made it public in `inventory.rs`! `pub struct InventoryUiRoot;`
    use cursed_warden::plugins::inventory::InventoryUiRoot;

    let root_entity = app.world_mut().query_filtered::<Entity, With<InventoryUiRoot>>().get_single(app.world());
    assert!(root_entity.is_ok(), "InventoryUiRoot should exist");
    let root_entity = root_entity.unwrap();

    let vis = app.world().get::<Visibility>(root_entity).unwrap();
    assert_eq!(vis, &Visibility::Visible);

    // Transition to NightPhase
    app.world_mut().resource_mut::<NextState<GameState>>().set(GameState::NightPhase);
    app.update(); // OnExit(Evening) runs here

    // Check UI is Hidden
    let vis = app.world().get::<Visibility>(root_entity).unwrap();
    assert_eq!(vis, &Visibility::Hidden);

    // Transition back to EveningPhase
    app.world_mut().resource_mut::<NextState<GameState>>().set(GameState::EveningPhase);
    app.update(); // OnEnter(Evening) runs here

    // Check UI is Visible and same entity (optional, but assumed)
    let vis = app.world().get::<Visibility>(root_entity).unwrap();
    assert_eq!(vis, &Visibility::Visible);
}
