use bevy::prelude::*;
use cursed_warden::plugins::core::GameState;
use cursed_warden::plugins::items::{ItemDatabase, ItemDefinition, ItemRarity, MaterialType, ItemType};
use cursed_warden::plugins::metagame::{PlayerStats, GlobalTime, PendingItems};
use cursed_warden::plugins::shop::{ShopPlugin, ShopState, ShopRerollEvent, BuyItemEvent, LockShopItemEvent};

#[test]
fn test_shop_logic_backend() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin); // Required for state transitions
    app.add_plugins(ShopPlugin);

    // Mock resources
    app.init_resource::<ItemDatabase>();
    app.init_resource::<PlayerStats>();
    app.init_resource::<GlobalTime>();
    app.init_resource::<PendingItems>();
    app.init_resource::<cursed_warden::plugins::inventory::InventoryGridState>();

    // Add GameState
    app.init_state::<GameState>();

    // Add some items to DB
    let mut item_db = app.world_mut().resource_mut::<ItemDatabase>();
    item_db.items.insert("common_sword".to_string(), ItemDefinition {
        id: "common_sword".to_string(),
        name: "Common Sword".to_string(),
        width: 1,
        height: 2,
        shape: vec![IVec2::new(0, 0), IVec2::new(0, 1)],
        material: MaterialType::Steel,
        item_type: ItemType::Weapon,
        tags: vec![],
        synergies: vec![],
        attack: 10.0,
        defense: 0.0,
        speed: 0.0,
        rarity: ItemRarity::Common,
        cost: 10,
    });
    item_db.items.insert("rare_sword".to_string(), ItemDefinition {
        id: "rare_sword".to_string(),
        name: "Rare Sword".to_string(),
        width: 1,
        height: 2,
        shape: vec![IVec2::new(0, 0), IVec2::new(0, 1)],
        material: MaterialType::Steel,
        item_type: ItemType::Weapon,
        tags: vec![],
        synergies: vec![],
        attack: 15.0,
        defense: 0.0,
        speed: 0.0,
        rarity: ItemRarity::Rare,
        cost: 20,
    });

    // Enter Evening Phase to trigger generation
    app.world_mut().resource_mut::<NextState<GameState>>().set(GameState::EveningPhase);
    app.update(); // State transition
    app.update(); // Systems run

    // Check generation
    let shop_state = app.world().resource::<ShopState>();
    assert_eq!(shop_state.offered_items.len(), 5);
    assert_eq!(shop_state.reroll_cost, 1);

    // Test Reroll Cost Logic
    app.world_mut().resource_mut::<PlayerStats>().thalers = 100;

    // 1st Reroll
    app.world_mut().send_event(ShopRerollEvent);
    app.update();
    let shop_state = app.world().resource::<ShopState>();
    assert_eq!(shop_state.reroll_count, 1);
    assert_eq!(shop_state.reroll_cost, 1);

    // 2nd
    app.world_mut().send_event(ShopRerollEvent);
    app.update();
    // 3rd
    app.world_mut().send_event(ShopRerollEvent);
    app.update();
    // 4th
    app.world_mut().send_event(ShopRerollEvent);
    app.update();

    let shop_state = app.world().resource::<ShopState>();
    assert_eq!(shop_state.reroll_count, 4);
    assert_eq!(shop_state.reroll_cost, 2); // Increased

    // Test Buying
    // Find a valid slot
    let shop_state = app.world().resource::<ShopState>();
    let mut valid_slot = 0;
    for (i, slot) in shop_state.offered_items.iter().enumerate() {
        if slot.is_some() { valid_slot = i; break; }
    }

    app.world_mut().send_event(BuyItemEvent { slot_index: valid_slot });
    app.update();

    let pending = app.world().resource::<PendingItems>();
    assert_eq!(pending.0.len(), 1);

    let shop_state = app.world().resource::<ShopState>();
    assert!(shop_state.offered_items[valid_slot].is_none());

    // Test Locking
    // Generate new items
    app.world_mut().send_event(ShopRerollEvent);
    app.update();

    let shop_state = app.world().resource::<ShopState>();
    // Find a slot
    let mut valid_slot = 0;
    let mut item_id = "".to_string();
    for (i, slot) in shop_state.offered_items.iter().enumerate() {
        if let Some(s) = slot {
            valid_slot = i;
            item_id = s.item_id.clone();
            break;
        }
    }

    // Lock it
    app.world_mut().send_event(LockShopItemEvent { slot_index: valid_slot });
    app.update();

    let shop_state = app.world().resource::<ShopState>();
    if let Some(s) = &shop_state.offered_items[valid_slot] {
        assert!(s.is_locked);
    } else {
        panic!("Slot should not be empty after locking");
    }

    // Reroll again
    app.world_mut().send_event(ShopRerollEvent);
    app.update();

    let shop_state = app.world().resource::<ShopState>();
    if let Some(s) = &shop_state.offered_items[valid_slot] {
        assert!(s.is_locked);
        assert_eq!(s.item_id, item_id); // Should be same item
    } else {
        panic!("Locked item disappeared");
    }
}
