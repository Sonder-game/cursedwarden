use bevy::prelude::*;
use rand::prelude::*;
use crate::plugins::items::{ItemDatabase, ItemDefinition, ItemRarity};
use crate::plugins::metagame::{PlayerStats, GlobalTime, PendingItems};
use crate::plugins::core::GameState;
use crate::plugins::inventory::{InventoryGridState, Item, CellState, ItemSpawnedEvent};

pub struct ShopPlugin;

impl Plugin for ShopPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShopState>()
           .add_event::<ShopRerollEvent>()
           .add_event::<BuyItemEvent>()
           .add_event::<SellItemEvent>()
           .add_event::<LockShopItemEvent>()
           .add_systems(OnEnter(GameState::EveningPhase), (generate_shop_items_on_entry, spawn_shop_ui).chain())
           .add_systems(OnExit(GameState::EveningPhase), cleanup_shop_ui)
           .add_systems(Update, (
               handle_shop_reroll,
               handle_buy_item,
               handle_sell_item,
               handle_lock_item,
               shop_interaction_system,
           ).run_if(in_state(GameState::EveningPhase)))
           .add_systems(Update, shop_ui_update_system.run_if(in_state(GameState::EveningPhase).and(resource_changed::<ShopState>)))
           .add_observer(handle_item_spawn_for_shop);
    }
}

#[derive(Resource, Default)]
pub struct ShopState {
    pub offered_items: Vec<Option<ShopSlot>>, // Fixed 5 slots
    pub reroll_cost: u32,
    pub reroll_count: u32,
    pub unique_generated_this_round: bool,
}

#[derive(Clone)]
pub struct ShopSlot {
    pub item_id: String,
    pub is_locked: bool,
    pub is_on_sale: bool, // 50% off
    pub original_cost: u32,
}

impl ShopSlot {
    pub fn current_cost(&self) -> u32 {
        if self.is_on_sale {
            (self.original_cost as f32 * 0.5).ceil() as u32
        } else {
            self.original_cost
        }
    }
}

#[derive(Event)]
pub struct ShopRerollEvent;

#[derive(Event)]
pub struct BuyItemEvent {
    pub slot_index: usize,
}

#[derive(Event)]
pub struct SellItemEvent {
    pub entity: Entity, // The item entity in inventory
    pub cost: u32,
}

#[derive(Event)]
pub struct LockShopItemEvent {
    pub slot_index: usize,
}

fn generate_shop_items_on_entry(
    mut shop_state: ResMut<ShopState>,
    item_db: Res<ItemDatabase>,
    global_time: Res<GlobalTime>,
) {
    shop_state.reroll_cost = 1;
    shop_state.reroll_count = 0;

    shop_state.unique_generated_this_round = false;

    if shop_state.offered_items.is_empty() {
        for _ in 0..5 {
            shop_state.offered_items.push(None);
        }
    }

    let mut slots_to_fill = Vec::new();
    for (i, slot) in shop_state.offered_items.iter().enumerate() {
        if let Some(s) = slot {
            if s.is_locked {
                continue;
            }
        }
        slots_to_fill.push(i);
    }

    let round = global_time.day;
    for i in slots_to_fill {
        shop_state.offered_items[i] = generate_single_item(&item_db, round, &mut shop_state.unique_generated_this_round, true);
    }
}

fn handle_shop_reroll(
    mut events: EventReader<ShopRerollEvent>,
    mut shop_state: ResMut<ShopState>,
    mut player_stats: ResMut<PlayerStats>,
    item_db: Res<ItemDatabase>,
    global_time: Res<GlobalTime>,
) {
    for _ in events.read() {
        if player_stats.thalers >= shop_state.reroll_cost {
            player_stats.thalers -= shop_state.reroll_cost;

            shop_state.reroll_count += 1;
            if shop_state.reroll_count >= 4 {
                shop_state.reroll_cost = 2;
            } else {
                shop_state.reroll_cost = 1;
            }

            let round = global_time.day;
            for i in 0..5 {
                let is_locked = if let Some(ref s) = shop_state.offered_items[i] {
                    s.is_locked
                } else {
                    false
                };

                if !is_locked {
                    shop_state.offered_items[i] = generate_single_item(&item_db, round, &mut shop_state.unique_generated_this_round, false);
                }
            }
            info!("Shop rerolled. New cost: {}", shop_state.reroll_cost);
        } else {
            warn!("Not enough thalers to reroll.");
        }
    }
}

fn handle_buy_item(
    mut events: EventReader<BuyItemEvent>,
    mut shop_state: ResMut<ShopState>,
    mut player_stats: ResMut<PlayerStats>,
    mut pending_items: ResMut<PendingItems>,
    _item_db: Res<ItemDatabase>,
) {
    for event in events.read() {
        let (cost, item_id) = if let Some(slot) = &shop_state.offered_items[event.slot_index] {
            (slot.current_cost(), slot.item_id.clone())
        } else {
            continue;
        };

        if player_stats.thalers >= cost {
            player_stats.thalers -= cost;
            pending_items.0.push(item_id.clone());

            shop_state.offered_items[event.slot_index] = None;

            info!("Bought item: {} for {}", item_id, cost);
        } else {
            warn!("Not enough thalers to buy item.");
        }
    }
}

fn handle_sell_item(
    mut events: EventReader<SellItemEvent>,
    mut commands: Commands,
    mut player_stats: ResMut<PlayerStats>,
    mut grid_state: ResMut<InventoryGridState>,
) {
    for event in events.read() {
        let mut cells_to_clear = Vec::new();
        for (pos, cell) in grid_state.grid.iter() {
             if let CellState::Occupied(occupier) = cell.state {
                 if occupier == event.entity {
                     cells_to_clear.push(*pos);
                 }
             }
        }
        for pos in cells_to_clear {
             if let Some(cell) = grid_state.grid.get_mut(&pos) {
                 cell.state = CellState::Free;
             }
        }

        commands.entity(event.entity).despawn_recursive();

        let refund = (event.cost as f32 * 0.5).ceil() as u32;
        player_stats.thalers += refund;

        info!("Sold item for {}", refund);
    }
}

// Observer for newly spawned items to detect drag-to-shop
fn handle_item_spawn_for_shop(
    trigger: Trigger<ItemSpawnedEvent>,
    mut commands: Commands,
) {
    let entity = trigger.event().0;
    commands.entity(entity).observe(handle_drag_drop_shop);
}

fn handle_drag_drop_shop(
    trigger: Trigger<Pointer<DragDrop>>,
    mut sell_ev: EventWriter<SellItemEvent>,
    q_items: Query<&ItemDefinition, With<Item>>,
    q_shop: Query<&ShopUiRoot>,
    q_parent: Query<&Parent>,
) {
    let dragged_entity = trigger.entity(); // Because we observe on the item
    let target_entity = trigger.event().dropped; // The entity we dropped onto

    // Check hierarchy for ShopUiRoot
    let mut current_entity = target_entity;
    loop {
        if q_shop.get(current_entity).is_ok() {
            // Found shop!
            if let Ok(def) = q_items.get(dragged_entity) {
                sell_ev.send(SellItemEvent {
                    entity: dragged_entity,
                    cost: def.cost,
                });
            }
            break;
        }

        if let Ok(parent) = q_parent.get(current_entity) {
            current_entity = parent.get();
        } else {
            break; // No more parents
        }
    }
}

fn handle_lock_item(
    mut events: EventReader<LockShopItemEvent>,
    mut shop_state: ResMut<ShopState>,
) {
    for event in events.read() {
        if let Some(slot) = &mut shop_state.offered_items[event.slot_index] {
            slot.is_locked = !slot.is_locked;
            info!("Toggled lock for slot {}: {}", event.slot_index, slot.is_locked);
        }
    }
}

fn generate_single_item(
    item_db: &ItemDatabase,
    round: u32,
    unique_generated: &mut bool,
    can_generate_unique: bool
) -> Option<ShopSlot> {
    let mut rng = rand::thread_rng();

    let rarity = determine_rarity(round, &mut rng);

    let try_unique = can_generate_unique && !*unique_generated && round >= 4;
    let final_rarity = if try_unique && rng.gen_bool(0.02) {
        *unique_generated = true;
        ItemRarity::Unique
    } else {
        rarity
    };

    let candidates: Vec<&ItemDefinition> = item_db.items.values()
        .filter(|item| item.rarity == final_rarity)
        .collect();

    if candidates.is_empty() {
        let common_candidates: Vec<&ItemDefinition> = item_db.items.values()
            .filter(|item| item.rarity == ItemRarity::Common)
            .collect();

        if common_candidates.is_empty() {
            return None;
        }
        let chosen = common_candidates.choose(&mut rng).unwrap();
        return Some(create_slot_from_item(chosen, &mut rng));
    }

    let chosen = candidates.choose(&mut rng).unwrap();
    Some(create_slot_from_item(chosen, &mut rng))
}

fn create_slot_from_item(item: &ItemDefinition, rng: &mut ThreadRng) -> ShopSlot {
    let is_sale = rng.gen_bool(0.10);
    ShopSlot {
        item_id: item.id.clone(),
        is_locked: false,
        is_on_sale: is_sale,
        original_cost: item.cost,
    }
}

fn determine_rarity(round: u32, rng: &mut ThreadRng) -> ItemRarity {
    let roll = rng.gen_range(0.0..100.0);

    if round < 8 {
        if roll < 70.0 { ItemRarity::Common }
        else if roll < 95.0 { ItemRarity::Rare }
        else { ItemRarity::Epic }
    } else if round < 14 {
        if roll < 40.0 { ItemRarity::Common }
        else if roll < 70.0 { ItemRarity::Rare }
        else if roll < 95.0 { ItemRarity::Epic }
        else { ItemRarity::Legendary }
    } else {
        if roll < 20.0 { ItemRarity::Common }
        else if roll < 45.0 { ItemRarity::Rare }
        else if roll < 75.0 { ItemRarity::Epic }
        else if roll < 95.0 { ItemRarity::Legendary }
        else { ItemRarity::Godly }
    }
}

// UI Components
#[derive(Component)]
struct ShopUiRoot;

#[derive(Component)]
struct ShopSlotUi {
    index: usize,
}

#[derive(Component)]
struct RerollButton;

#[derive(Component)]
struct RerollCostText;

#[derive(Component)]
struct LockButton {
    slot_index: usize,
}

#[derive(Component)]
struct BuyButton {
    slot_index: usize,
}

// UI Systems

fn spawn_shop_ui(mut commands: Commands) {
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(200.0),
            display: Display::Flex,
            justify_content: JustifyContent::SpaceEvenly,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Row,
            position_type: PositionType::Absolute,
            top: Val::Px(0.0),
            ..default()
        },
        BackgroundColor(Color::srgb(0.15, 0.15, 0.2)),
        ShopUiRoot,
        PickingBehavior::default(), // Ensure it can be picked as a drop target
    ))
    .with_children(|parent| {
        parent.spawn(Node {
            width: Val::Px(100.0),
            height: Val::Percent(80.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        }).with_children(|p| {
             p.spawn((
                Button,
                Node {
                    width: Val::Px(80.0),
                    height: Val::Px(50.0),
                    border: UiRect::all(Val::Px(2.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BorderColor(Color::BLACK),
                BackgroundColor(Color::srgb(0.6, 0.4, 0.2)),
                RerollButton,
             )).with_children(|btn| {
                 btn.spawn((
                     Text::new("Reroll"),
                     TextFont { font_size: 16.0, ..default() },
                     TextColor(Color::WHITE),
                 ));
             });

             p.spawn((
                 Text::new("Cost: 1"),
                 TextFont { font_size: 14.0, ..default() },
                 TextColor(Color::srgb(1.0, 1.0, 0.0)), // YELLOW
                 RerollCostText,
             ));
        });

        for i in 0..5 {
            parent.spawn((
                Node {
                    width: Val::Px(120.0),
                    height: Val::Px(160.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(2.0)),
                    padding: UiRect::all(Val::Px(5.0)),
                    margin: UiRect::all(Val::Px(5.0)),
                    ..default()
                },
                BorderColor(Color::srgb(0.5, 0.5, 0.5)), // GRAY
                BackgroundColor(Color::srgb(0.2, 0.2, 0.25)),
                ShopSlotUi { index: i },
            ));
        }
    });
}

fn cleanup_shop_ui(mut commands: Commands, q_root: Query<Entity, With<ShopUiRoot>>) {
    for e in q_root.iter() {
        commands.entity(e).despawn_recursive();
    }
}

fn shop_ui_update_system(
    mut commands: Commands,
    shop_state: Res<ShopState>,
    item_db: Res<ItemDatabase>,
    q_slots: Query<(Entity, &ShopSlotUi)>,
    mut q_reroll_text: Query<&mut Text, With<RerollCostText>>,
) {
    for mut text in q_reroll_text.iter_mut() {
        text.0 = format!("Cost: {}", shop_state.reroll_cost);
    }

    for (entity, slot_ui) in q_slots.iter() {
        commands.entity(entity).despawn_descendants();

        let slot_data = &shop_state.offered_items[slot_ui.index];

        if let Some(data) = slot_data {
            if let Some(def) = item_db.items.get(&data.item_id) {
                 commands.entity(entity).with_children(|parent| {
                     parent.spawn((
                         Text::new(&def.name),
                         TextFont { font_size: 14.0, ..default() },
                         TextColor(match def.rarity {
                             ItemRarity::Common => Color::WHITE,
                             ItemRarity::Rare => Color::srgb(0.4, 0.4, 1.0),
                             ItemRarity::Epic => Color::srgb(0.8, 0.0, 0.8),
                             ItemRarity::Legendary => Color::srgb(1.0, 0.6, 0.0),
                             ItemRarity::Godly => Color::srgb(1.0, 0.0, 0.0), // RED
                             ItemRarity::Unique => Color::srgb(0.0, 1.0, 1.0),
                         }),
                     ));

                     parent.spawn((
                         Node {
                             width: Val::Px(60.0),
                             height: Val::Px(60.0),
                             margin: UiRect::all(Val::Px(5.0)),
                             ..default()
                         },
                         BackgroundColor(match def.rarity {
                             ItemRarity::Common => Color::srgb(0.5, 0.5, 0.5),
                             _ => Color::srgb(0.6, 0.6, 0.7),
                         }),
                     ));

                     if data.is_on_sale {
                         parent.spawn((
                             Text::new("SALE -50%"),
                             TextFont { font_size: 12.0, ..default() },
                             TextColor(Color::srgb(0.0, 1.0, 0.0)), // GREEN
                         ));
                     }

                     parent.spawn(Node {
                             width: Val::Percent(100.0),
                             flex_direction: FlexDirection::Row,
                             justify_content: JustifyContent::SpaceBetween,
                             align_items: AlignItems::Center,
                             ..default()
                         })
                     .with_children(|controls| {
                         controls.spawn((
                             Button,
                             Node {
                                 width: Val::Px(50.0),
                                 height: Val::Px(30.0),
                                 justify_content: JustifyContent::Center,
                                 align_items: AlignItems::Center,
                                 ..default()
                             },
                             BackgroundColor(Color::srgb(0.2, 0.5, 0.2)),
                             BuyButton { slot_index: slot_ui.index },
                         )).with_children(|b| {
                             b.spawn((
                                 Text::new(format!("{}", data.current_cost())),
                                 TextFont { font_size: 14.0, ..default() },
                                 TextColor(Color::WHITE),
                             ));
                         });

                         controls.spawn((
                             Button,
                             Node {
                                 width: Val::Px(30.0),
                                 height: Val::Px(30.0),
                                 justify_content: JustifyContent::Center,
                                 align_items: AlignItems::Center,
                                 ..default()
                             },
                             BackgroundColor(if data.is_locked { Color::srgb(1.0, 0.0, 0.0) } else { Color::srgb(0.5, 0.5, 0.5) }),
                             LockButton { slot_index: slot_ui.index },
                         )).with_children(|b| {
                             b.spawn((
                                 Text::new(if data.is_locked { "L" } else { "U" }),
                                 TextFont { font_size: 14.0, ..default() },
                                 TextColor(Color::WHITE),
                             ));
                         });
                     });
                 });
            }
        } else {
            commands.entity(entity).with_children(|parent| {
                 parent.spawn((
                     Text::new("Sold Out"),
                     TextFont { font_size: 14.0, ..default() },
                     TextColor(Color::srgb(0.5, 0.5, 0.5)), // GRAY
                 ));
            });
        }
    }
}

fn shop_interaction_system(
    mut interaction_query: Query<
        (&Interaction, Option<&RerollButton>, Option<&BuyButton>, Option<&LockButton>),
        (Changed<Interaction>, With<Button>),
    >,
    mut reroll_ev: EventWriter<ShopRerollEvent>,
    mut buy_ev: EventWriter<BuyItemEvent>,
    mut lock_ev: EventWriter<LockShopItemEvent>,
) {
    for (interaction, reroll, buy, lock) in interaction_query.iter_mut() {
        if *interaction == Interaction::Pressed {
            if reroll.is_some() {
                reroll_ev.send(ShopRerollEvent);
            } else if let Some(b) = buy {
                buy_ev.send(BuyItemEvent { slot_index: b.slot_index });
            } else if let Some(l) = lock {
                lock_ev.send(LockShopItemEvent { slot_index: l.slot_index });
            }
        }
    }
}
