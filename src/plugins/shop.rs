use bevy::prelude::*;
use rand::Rng;
use crate::plugins::items::{ItemDatabase, ItemDefinition, ItemRarity};
use crate::plugins::metagame::{PlayerStats, GlobalTime};
use crate::plugins::inventory::{InventoryGridState, spawn_item_entity, InventoryGridContainer};
use crate::plugins::core::GameState;

pub struct ShopPlugin;

impl Plugin for ShopPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShopState>()
           .add_systems(OnEnter(GameState::EveningPhase), on_enter_shop)
           .add_systems(OnExit(GameState::EveningPhase), cleanup_shop_ui)
           .add_systems(Update, (
               reroll_button_system,
               buy_item_system,
               lock_item_system,
               update_shop_ui_system
           ).run_if(in_state(GameState::EveningPhase)));
    }
}

#[derive(Debug, Clone)]
pub struct ShopItem {
    pub item_id: String,
    pub price: u32,
    pub is_locked: bool,
    pub is_discounted: bool,
    pub is_sold: bool,
}

#[derive(Resource, Default)]
pub struct ShopState {
    pub items: Vec<ShopItem>, // Fixed size of 5
    pub reroll_cost: u32,
    pub reroll_count: u32,
}

#[derive(Component)]
struct ShopUiRoot;

#[derive(Component)]
struct RerollButton;

#[derive(Component)]
struct ShopSlot(#[allow(dead_code)] usize);

#[derive(Component)]
struct LockButton(usize);

#[derive(Component)]
struct BuyButton(usize);

fn on_enter_shop(
    mut shop_state: ResMut<ShopState>,
    item_db: Res<ItemDatabase>,
    global_time: Res<GlobalTime>,
    mut commands: Commands,
) {
    // Reset reroll cost at start of round
    shop_state.reroll_cost = 1;
    shop_state.reroll_count = 0;

    // Generate initial shop items (respecting locks would happen if we persisted ShopState between rounds,
    // but typically ShopState is refreshed per round.
    // However, the prompt says "Lock item... prevents replacement".
    // This implies ShopState should persist or we need to handle "next round generation" carefully.
    // For now, let's assume if items are empty, we generate. If not empty (from previous round?), we respect locks?
    // Actually, "EveningPhase" is the shop phase. When we enter it, we are starting a shopping session.
    // If we come from "DayPhase", it's a new round.
    // If we come from "NightPhase" (after combat), it might be the same round or next?
    // Typically: Shop -> Combat -> Shop (Next Round).
    // So on Enter EveningPhase, we should refresh the shop.

    // Let's implement generation logic.
    // We need to keep locked items.
    // let mut new_items: Vec<ShopItem> = Vec::new(); // Unused

    // Check if we have existing items (from previous round)
    // If shop_state.items is empty, we just generate 5.
    // If not empty, we keep locked ones.

    // BUT: On first load, it is empty.
    // On subsequent rounds, we might want to keep locked items.
    // However, `ShopState` is a Resource, so it persists.

    // We need to know if this is a "New Round".
    // GlobalTime.day increments. We can use that?
    // Let's assume OnEnter EveningPhase is always a "new shopping session".
    // We should probably clear non-locked items and refill.

    let round = global_time.day;

    // Identify indices of locked items
    let mut locked_items = Vec::new();
    if !shop_state.items.is_empty() {
        for item in &shop_state.items {
            if item.is_locked && !item.is_sold {
                locked_items.push(item.clone());
            }
        }
    }

    shop_state.items.clear();

    // Add locked items back
    for item in locked_items {
        shop_state.items.push(item);
    }

    // Fill the rest
    let needed = 5 - shop_state.items.len();
    if needed > 0 {
         let generated = generate_shop_items(&item_db, round, needed, true);
         shop_state.items.extend(generated);
    }

    spawn_shop_ui(&mut commands, &shop_state, &item_db);
}

pub fn generate_shop_items(
    item_db: &ItemDatabase,
    round: u32,
    count: usize,
    is_start_of_round: bool
) -> Vec<ShopItem> {
    let mut rng = rand::thread_rng();
    let mut results = Vec::new();

    for _ in 0..count {
        // 1. Determine Rarity
        let rarity = roll_rarity(round, &mut rng, is_start_of_round);

        // 2. Pick item of that rarity
        let candidates: Vec<&ItemDefinition> = item_db.items.values()
            .filter(|i| i.rarity == rarity)
            .collect();

        if let Some(choice) = pick_random(&candidates, &mut rng) {
             // 3. Determine Sale
             let is_discounted = rng.gen_bool(0.10); // 10% chance
             let mut price = choice.price;
             if is_discounted {
                 price = (price as f32 * 0.5).ceil() as u32;
             }

             results.push(ShopItem {
                 item_id: choice.id.clone(),
                 price,
                 is_locked: false,
                 is_discounted,
                 is_sold: false,
             });
        } else {
            // Fallback if no item of rarity found (e.g. no Unique items defined yet)
             // Try Common
             if let Some(fallback) = item_db.items.values().filter(|i| i.rarity == ItemRarity::Common).next() {
                  results.push(ShopItem {
                     item_id: fallback.id.clone(),
                     price: fallback.price,
                     is_locked: false,
                     is_discounted: false,
                     is_sold: false,
                 });
             }
        }
    }

    results
}

pub fn roll_rarity(round: u32, rng: &mut impl Rng, is_start_of_round: bool) -> ItemRarity {
    // Unique check: 2% chance at start of round, starting from round 4
    if is_start_of_round && round >= 4 {
        if rng.gen_bool(0.02) {
            return ItemRarity::Unique;
        }
    }

    // Weights based on round
    // Simplified progression logic:
    // Rounds 1-3: Common (80%), Rare (20%)
    // Rounds 4-7: Common (60%), Rare (30%), Epic (10%)
    // Rounds 8-10: Common (40%), Rare (30%), Epic (25%), Legendary (5%)
    // Rounds 11+: Common (20%), Rare (30%), Epic (30%), Legendary (15%), Godly (5%)

    let (common, rare, epic, legendary, godly) = if round <= 3 {
        (80, 20, 0, 0, 0)
    } else if round <= 7 {
        (60, 30, 10, 0, 0)
    } else if round <= 10 {
        (40, 30, 25, 5, 0)
    } else {
        (20, 30, 30, 15, 5)
    };

    let total = common + rare + epic + legendary + godly;
    let roll = rng.gen_range(0..total);

    if roll < common { ItemRarity::Common }
    else if roll < common + rare { ItemRarity::Rare }
    else if roll < common + rare + epic { ItemRarity::Epic }
    else if roll < common + rare + epic + legendary { ItemRarity::Legendary }
    else { ItemRarity::Godly }
}

pub fn pick_random<'a, T>(list: &'a Vec<T>, rng: &mut impl Rng) -> Option<&'a T> {
    if list.is_empty() { return None; }
    let idx = rng.gen_range(0..list.len());
    Some(&list[idx])
}

// UI Systems

fn spawn_shop_ui(
    commands: &mut Commands,
    shop_state: &ShopState,
    item_db: &ItemDatabase,
) {
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(30.0), // Shop at bottom or top? Let's put it at top 30%
            position_type: PositionType::Absolute,
            top: Val::Px(0.0),
            left: Val::Px(0.0),
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceEvenly,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(10.0)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.2, 0.15, 0.1)),
        ShopUiRoot,
    ))
    .with_children(|parent| {
        // Reroll Button / Info
        parent.spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                ..default()
            },
        )).with_children(|p| {
             p.spawn((
                Button,
                Node {
                    width: Val::Px(80.0),
                    height: Val::Px(50.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.6, 0.4, 0.2)),
                RerollButton,
            )).with_children(|btn| {
                btn.spawn((
                    Text::new(format!("Reroll\n{}g", shop_state.reroll_cost)),
                    TextFont { font_size: 16.0, ..default() },
                    TextColor(Color::WHITE),
                ));
            });
        });

        // Shop Slots
        for (i, item) in shop_state.items.iter().enumerate() {
            if let Some(def) = item_db.items.get(&item.item_id) {
                let bg_color = if item.is_sold {
                    Color::srgba(0.1, 0.1, 0.1, 0.5)
                } else if item.is_locked {
                    Color::srgb(0.3, 0.3, 0.5) // Blueish for locked
                } else {
                    Color::srgb(0.4, 0.3, 0.2)
                };

                parent.spawn((
                    Node {
                        width: Val::Px(100.0),
                        height: Val::Px(140.0),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        margin: UiRect::all(Val::Px(5.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(bg_color),
                    BorderColor(if item.is_discounted { Color::srgb(1.0, 0.8, 0.0) } else { Color::BLACK }), // Gold border for discount
                    ShopSlot(i),
                )).with_children(|slot| {
                    // Item Name
                    slot.spawn((
                        Text::new(&def.name),
                        TextFont { font_size: 14.0, ..default() },
                        TextColor(Color::WHITE),
                    ));

                    // Price
                    slot.spawn((
                        Text::new(format!("{}g", item.price)),
                        TextFont { font_size: 16.0, ..default() },
                        TextColor(if item.is_discounted { Color::srgb(0.0, 1.0, 0.0) } else { Color::WHITE }),
                    ));

                     // Buy Button (if not sold)
                     if !item.is_sold {
                         slot.spawn((
                            Button,
                            Node {
                                width: Val::Px(80.0),
                                height: Val::Px(30.0),
                                margin: UiRect::top(Val::Px(10.0)),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.2, 0.6, 0.2)),
                            BuyButton(i),
                        )).with_children(|btn| {
                            btn.spawn((
                                Text::new("Buy"),
                                TextFont { font_size: 14.0, ..default() },
                                TextColor(Color::WHITE),
                            ));
                        });

                         // Lock Button
                         slot.spawn((
                            Button,
                            Node {
                                width: Val::Px(80.0),
                                height: Val::Px(20.0),
                                margin: UiRect::top(Val::Px(5.0)),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(if item.is_locked { Color::srgb(0.3, 0.3, 0.8) } else { Color::srgb(0.4, 0.4, 0.4) }),
                            LockButton(i),
                        )).with_children(|btn| {
                            btn.spawn((
                                Text::new(if item.is_locked { "Unlock" } else { "Lock" }),
                                TextFont { font_size: 12.0, ..default() },
                                TextColor(Color::WHITE),
                            ));
                        });
                     } else {
                         slot.spawn((
                             Text::new("SOLD"),
                             TextFont { font_size: 20.0, ..default() },
                             TextColor(Color::srgb(0.5, 0.5, 0.5)),
                         ));
                     }
                });
            }
        }
    });
}

fn cleanup_shop_ui(mut commands: Commands, q_root: Query<Entity, With<ShopUiRoot>>) {
    for e in q_root.iter() {
        commands.entity(e).despawn_recursive();
    }
}

// Interactivity Systems

fn reroll_button_system(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<RerollButton>),
    >,
    mut shop_state: ResMut<ShopState>,
    mut player_stats: ResMut<PlayerStats>,
    global_time: Res<GlobalTime>,
    item_db: Res<ItemDatabase>,
    mut commands: Commands,
    q_root: Query<Entity, With<ShopUiRoot>>,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = BackgroundColor(Color::srgb(0.35, 0.75, 0.35));

                if player_stats.thalers >= shop_state.reroll_cost {
                    // Deduct cost
                    player_stats.thalers -= shop_state.reroll_cost;

                    // Increment cost logic: 1 gold for first 4, then 2.
                    shop_state.reroll_count += 1;
                    if shop_state.reroll_count >= 4 {
                        shop_state.reroll_cost = 2;
                    } else {
                         shop_state.reroll_cost = 1;
                    }

                    // Reroll Logic
                    let mut new_items = Vec::new();

                    // Keep locked
                     for item in &shop_state.items {
                        if item.is_locked && !item.is_sold {
                            new_items.push(item.clone());
                        }
                    }

                    // Generate rest
                    let needed = 5 - new_items.len();
                    if needed > 0 {
                        // Not start of round, so no Unique check
                        let generated = generate_shop_items(&item_db, global_time.day, needed, false);
                        new_items.extend(generated);
                    }

                    shop_state.items = new_items;

                    // Refresh UI
                    if let Ok(root) = q_root.get_single() {
                        commands.entity(root).despawn_recursive();
                        spawn_shop_ui(&mut commands, &shop_state, &item_db);
                    }
                }
            }
            Interaction::Hovered => {
                *color = BackgroundColor(Color::srgb(0.7, 0.5, 0.3));
            }
            Interaction::None => {
                *color = BackgroundColor(Color::srgb(0.6, 0.4, 0.2));
            }
        }
    }
}

fn lock_item_system(
    mut interaction_query: Query<
        (&Interaction, &LockButton),
        (Changed<Interaction>, With<Button>),
    >,
    mut shop_state: ResMut<ShopState>,
    item_db: Res<ItemDatabase>,
    mut commands: Commands,
    q_root: Query<Entity, With<ShopUiRoot>>,
) {
     for (interaction, lock_btn) in &mut interaction_query {
        if *interaction == Interaction::Pressed {
            let index = lock_btn.0;
            if index < shop_state.items.len() {
                shop_state.items[index].is_locked = !shop_state.items[index].is_locked;

                // Refresh UI to show status
                if let Ok(root) = q_root.get_single() {
                    commands.entity(root).despawn_recursive();
                    spawn_shop_ui(&mut commands, &shop_state, &item_db);
                }
            }
        }
    }
}

fn buy_item_system(
    mut interaction_query: Query<
        (&Interaction, &BuyButton),
        (Changed<Interaction>, With<Button>),
    >,
    mut shop_state: ResMut<ShopState>,
    mut player_stats: ResMut<PlayerStats>,
    mut grid_state: ResMut<InventoryGridState>,
    item_db: Res<ItemDatabase>,
    mut commands: Commands,
    q_root: Query<Entity, With<ShopUiRoot>>,
    q_container: Query<Entity, With<InventoryGridContainer>>,
    _pending_items: ResMut<crate::plugins::metagame::PendingItems>,
) {
    for (interaction, buy_btn) in &mut interaction_query {
        if *interaction == Interaction::Pressed {
            let index = buy_btn.0;
            if index < shop_state.items.len() {
                let item = &mut shop_state.items[index];
                if !item.is_sold && player_stats.thalers >= item.price {
                     if let Some(def) = item_db.items.get(&item.item_id) {
                         // Check if space exists
                         if let Some(pos) = grid_state.find_free_spot(def) {
                             // Buy successful
                             player_stats.thalers -= item.price;
                             item.is_sold = true;

                             // Spawn item directly into grid
                             if let Ok(container) = q_container.get_single() {
                                 spawn_item_entity(
                                     &mut commands,
                                     container,
                                     def,
                                     pos,
                                     0, // Initial rotation
                                     &mut grid_state
                                 );
                             }

                              // Refresh UI
                            if let Ok(root) = q_root.get_single() {
                                commands.entity(root).despawn_recursive();
                                spawn_shop_ui(&mut commands, &shop_state, &item_db);
                            }
                         } else {
                             // Feedback: No Space
                             info!("No space for item!");
                         }
                     }
                }
            }
        }
    }
}

fn update_shop_ui_system() {
    // Just a placeholder if we want animations
}
