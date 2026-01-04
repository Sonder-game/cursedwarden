use bevy::prelude::*;
use bevy::state::prelude::*;
use bevy::utils::HashMap;
use crate::plugins::core::GameState;
use crate::plugins::items::{ItemDatabase, ItemDefinition};
use crate::plugins::metagame::{PersistentInventory, SavedItem, PlayerStats};
use rand::Rng;

pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InventoryGridState>()
           .init_resource::<ShopState>()
           .add_systems(OnEnter(GameState::EveningPhase), (spawn_inventory_ui, apply_deferred, load_inventory_state, apply_deferred, consume_pending_items, populate_shop_on_start).chain())
           .add_systems(OnExit(GameState::EveningPhase), (save_inventory_state, cleanup_inventory_ui).chain())
           .add_systems(Update, (resize_item_system, debug_spawn_item_system, refresh_shop_ui, update_thalers_ui, shop_reroll_interaction).run_if(in_state(GameState::EveningPhase)))
           .add_systems(OnEnter(GameState::NightPhase), crate::plugins::mutation::mutation_system)
           .add_observer(attach_drag_observers);
    }
}

// Event triggered when an item is spawned (e.g. from load) and needs interactivity
#[derive(Event)]
pub struct ItemSpawnedEvent(pub Entity);

// Components
#[derive(Component, Debug, Clone, Copy)]
pub struct InventorySlot {
    pub x: i32,
    pub y: i32,
}

#[derive(Component)]
pub struct InventoryGridContainer;

#[derive(Component)]
pub struct ShopContainer;

#[derive(Component)]
pub struct ThalersText;

#[derive(Component)]
pub struct RerollButton;

#[derive(Component)]
pub struct Item;

#[derive(Component)]
pub struct ShopItem {
    pub slot_index: usize,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct GridPosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct ItemSize {
    pub width: i32,
    pub height: i32,
}

#[derive(Component)]
pub struct DragOriginalPosition {
    pub parent: Entity,
    pub left: Val,
    pub top: Val,
    pub z_index: ZIndex,
}

// Resources
#[derive(Resource)]
pub struct InventoryGridState {
   pub cells: HashMap<IVec2, Entity>,
   pub width: i32,
   pub height: i32,
}

impl Default for InventoryGridState {
    fn default() -> Self {
        Self {
            cells: HashMap::new(),
            width: 8,
            height: 8,
        }
    }
}

impl InventoryGridState {
    pub fn is_area_free(&self, pos: IVec2, size: ItemSize, exclude_entity: Option<Entity>) -> bool {
        // Check bounds
        if pos.x < 0 || pos.y < 0 || pos.x + size.width > self.width || pos.y + size.height > self.height {
            return false;
        }

        // Check collisions
        for dy in 0..size.height {
            for dx in 0..size.width {
                let check_pos = IVec2::new(pos.x + dx, pos.y + dy);
                if let Some(occupier) = self.cells.get(&check_pos) {
                    if Some(*occupier) != exclude_entity {
                        return false;
                    }
                }
            }
        }
        true
    }

    pub fn find_free_spot(&self, size: ItemSize) -> Option<IVec2> {
        for y in 0..self.height {
            for x in 0..self.width {
                let pos = IVec2::new(x, y);
                if self.is_area_free(pos, size, None) {
                    return Some(pos);
                }
            }
        }
        None
    }
}

#[derive(Resource, Default)]
pub struct ShopState {
    pub items: Vec<Option<ItemDefinition>>, // Fixed slots
    pub reroll_cost: u32,
}

// Systems
fn resize_item_system(
    mut q_items: Query<(&mut Node, &ItemSize), Changed<ItemSize>>,
) {
    for (mut node, size) in q_items.iter_mut() {
        // 50px per slot + (size-1) * 2px gaps
        let width = size.width as f32 * 50.0 + (size.width - 1) as f32 * 2.0;
        let height = size.height as f32 * 50.0 + (size.height - 1) as f32 * 2.0;
        node.width = Val::Px(width);
        node.height = Val::Px(height);
    }
}

fn spawn_inventory_ui(mut commands: Commands, mut grid_state: ResMut<InventoryGridState>) {
    grid_state.cells.clear();

    // Root Node (Split Screen)
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                display: Display::Flex,
                flex_direction: FlexDirection::Row, // Split Left/Right
                justify_content: JustifyContent::SpaceEvenly,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.05, 0.05, 0.1)),
            InventoryUiRoot,
        ))
        .with_children(|root| {
            // LEFT: Inventory Grid
            root.spawn((
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    ..default()
                },
            )).with_children(|col| {
                 col.spawn((
                     Text::new("Inventory"),
                     TextFont { font_size: 24.0, ..default() },
                     TextColor(Color::WHITE),
                     Node { margin: UiRect::bottom(Val::Px(10.0)), ..default() }
                 ));

                 col.spawn((
                    InventoryGridContainer,
                    Node {
                        display: Display::Grid,
                        grid_template_columns: vec![GridTrack::px(50.0); grid_state.width as usize],
                        grid_template_rows: vec![GridTrack::px(50.0); grid_state.height as usize],
                        row_gap: Val::Px(2.0),
                        column_gap: Val::Px(2.0),
                        padding: UiRect::all(Val::Px(10.0)),
                        position_type: PositionType::Relative,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
                ))
                .with_children(|grid_parent| {
                    // Spawn Slots
                    for y in 0..grid_state.height {
                        for x in 0..grid_state.width {
                           grid_parent.spawn((
                                Node {
                                    width: Val::Px(50.0),
                                    height: Val::Px(50.0),
                                    border: UiRect::all(Val::Px(1.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
                                BorderColor(Color::BLACK),
                                InventorySlot { x, y },
                            ));
                        }
                    }
                });
            });

            // RIGHT: Shop Panel
            root.spawn((
                Node {
                    width: Val::Px(300.0),
                    height: Val::Percent(80.0),
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(20.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BorderColor(Color::srgb(0.6, 0.4, 0.2)),
                BackgroundColor(Color::srgb(0.15, 0.1, 0.05)),
            )).with_children(|shop| {
                // Header
                shop.spawn((
                    Text::new("Shop"),
                    TextFont { font_size: 28.0, ..default() },
                    TextColor(Color::srgb(1.0, 0.84, 0.0)),
                    Node { margin: UiRect::bottom(Val::Px(10.0)), ..default() }
                ));

                shop.spawn((
                    Text::new("Thalers: 0"),
                    TextFont { font_size: 20.0, ..default() },
                    TextColor(Color::WHITE),
                    Node { margin: UiRect::bottom(Val::Px(20.0)), ..default() },
                    ThalersText,
                ));

                // Shop Items Container
                shop.spawn((
                    ShopContainer,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(400.0), // Fixed height area for items
                        margin: UiRect::bottom(Val::Px(20.0)),
                        // We use relative positioning for children so they can be dragged freely initially?
                        // Or just Flex column? Let's use Flex Column for slots.
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(10.0),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.3)),
                ));

                // Controls
                shop.spawn((
                    Button,
                    Node {
                        width: Val::Px(150.0),
                        height: Val::Px(50.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.3, 0.5, 0.3)),
                    RerollButton,
                )).with_children(|btn| {
                    btn.spawn((
                        Text::new("Reroll (1g)"),
                        TextFont { font_size: 18.0, ..default() },
                        TextColor(Color::WHITE),
                    ));
                });
            });
        });
}

#[derive(Component)]
struct InventoryUiRoot;

fn cleanup_inventory_ui(
    mut commands: Commands,
    q_root: Query<Entity, With<InventoryUiRoot>>,
) {
    for entity in q_root.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn populate_shop_on_start(
    mut shop_state: ResMut<ShopState>,
    item_db: Res<ItemDatabase>,
) {
    if shop_state.items.is_empty() {
        shop_state.reroll_cost = 1;
        shop_state.items = vec![None; 3]; // 3 Shop Slots
        roll_shop(&mut shop_state, &item_db);
    }
}

fn roll_shop(shop_state: &mut ShopState, item_db: &ItemDatabase) {
    let mut rng = rand::thread_rng();
    let keys: Vec<&String> = item_db.items.keys().collect();
    if keys.is_empty() { return; }

    for i in 0..shop_state.items.len() {
        let random_key = keys[rng.gen_range(0..keys.len())];
        if let Some(def) = item_db.items.get(random_key) {
            shop_state.items[i] = Some(def.clone());
        }
    }
}

fn shop_reroll_interaction(
    mut commands: Commands,
    mut q_interaction: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<RerollButton>)>,
    mut shop_state: ResMut<ShopState>,
    mut player_stats: ResMut<PlayerStats>,
    item_db: Res<ItemDatabase>,
    q_shop_items: Query<Entity, With<ShopItem>>,
) {
    for (interaction, mut bg) in q_interaction.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                if player_stats.thalers >= shop_state.reroll_cost {
                    player_stats.thalers -= shop_state.reroll_cost;

                    // Clear current shop entities
                    for entity in q_shop_items.iter() {
                        commands.entity(entity).despawn_recursive();
                    }

                    roll_shop(&mut shop_state, &item_db);
                    *bg = BackgroundColor(Color::srgb(0.2, 0.4, 0.2));
                }
            },
            Interaction::Hovered => *bg = BackgroundColor(Color::srgb(0.4, 0.6, 0.4)),
            Interaction::None => *bg = BackgroundColor(Color::srgb(0.3, 0.5, 0.3)),
        }
    }
}

fn refresh_shop_ui(
    mut commands: Commands,
    shop_state: Res<ShopState>,
    q_container: Query<Entity, With<ShopContainer>>,
    q_existing_items: Query<&ShopItem>,
    // Only run if shop state changed? We can just check if container is empty or mismatched
) {
    if let Ok(container) = q_container.get_single() {
        // Simple hack: if container is empty but state has items, spawn them.
        // Real implementation should be more reactive.
        let existing_count = q_existing_items.iter().count();
        if existing_count == 0 && shop_state.items.iter().any(|i| i.is_some()) {
             for (idx, item_opt) in shop_state.items.iter().enumerate() {
                 if let Some(def) = item_opt {
                    spawn_shop_item(&mut commands, container, def, idx);
                 }
             }
        }
    }
}

fn spawn_shop_item(commands: &mut Commands, container: Entity, def: &ItemDefinition, index: usize) {
     let size = ItemSize { width: def.width as i32, height: def.height as i32 };
     // Calculate pixel size
     let width = size.width as f32 * 50.0 + (size.width - 1) as f32 * 2.0;
     let height = size.height as f32 * 50.0 + (size.height - 1) as f32 * 2.0;

     let item_entity = commands.spawn((
        Node {
            width: Val::Px(width),
            height: Val::Px(height),
            margin: UiRect::bottom(Val::Px(10.0)), // Spacing in shop list
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.4, 0.2, 0.2)), // Reddish for shop items
        BorderColor(Color::srgb(1.0, 0.84, 0.0)),
        Interaction::default(),
        ShopItem { slot_index: index },
        size,
        def.clone(),
    ))
    .with_children(|parent| {
         parent.spawn((
             Text::new(format!("{}\n{}g", def.name, def.cost)),
             TextFont { font_size: 14.0, ..default() },
             TextColor(Color::WHITE),
             Node {
                 position_type: PositionType::Absolute,
                 left: Val::Px(2.0),
                 top: Val::Px(2.0),
                 ..default()
             },
             PickingBehavior::IGNORE,
         ));
    })
    .observe(handle_drag_start)
    .observe(handle_drag)
    .observe(handle_drag_drop)
    .observe(handle_drag_end)
    .id();

    commands.entity(container).add_child(item_entity);
}

fn update_thalers_ui(
    player_stats: Res<PlayerStats>,
    mut q_text: Query<&mut Text, With<ThalersText>>,
) {
    for mut text in q_text.iter_mut() {
        text.0 = format!("Thalers: {}", player_stats.thalers);
    }
}

// ====================
// Persistence Logic
// ====================

fn save_inventory_state(
    mut persistent_inventory: ResMut<PersistentInventory>,
    q_items: Query<(&ItemDefinition, &GridPosition), With<Item>>,
) {
    let mut saved_items = Vec::new();
    for (def, pos) in q_items.iter() {
        saved_items.push(SavedItem {
            item_id: def.id.clone(),
            grid_x: pos.x,
            grid_y: pos.y,
        });
    }
    persistent_inventory.items = saved_items;
    info!("Saved {} items to persistent inventory state", persistent_inventory.items.len());
}

fn load_inventory_state(
    mut commands: Commands,
    persistent_inventory: Res<PersistentInventory>,
    mut grid_state: ResMut<InventoryGridState>,
    item_db: Res<ItemDatabase>,
    q_container: Query<Entity, With<InventoryGridContainer>>,
) {
    if let Ok(container) = q_container.get_single() {
        for saved_item in &persistent_inventory.items {
            if let Some(def) = item_db.items.get(&saved_item.item_id) {
                 let size = ItemSize { width: def.width as i32, height: def.height as i32 };
                 let pos = IVec2::new(saved_item.grid_x, saved_item.grid_y);

                 if grid_state.is_area_free(pos, size, None) {
                     spawn_item_entity(
                         &mut commands,
                         container,
                         def,
                         pos,
                         size,
                         &mut grid_state
                     );
                 } else {
                     warn!("Could not restore item {} at {:?}: Space occupied", def.name, pos);
                 }
            }
        }
    }
}

fn consume_pending_items(
    mut commands: Commands,
    mut pending_items: ResMut<crate::plugins::metagame::PendingItems>,
    mut grid_state: ResMut<InventoryGridState>,
    item_db: Res<ItemDatabase>,
    q_container: Query<Entity, With<InventoryGridContainer>>,
) {
    if let Ok(container) = q_container.get_single() {
        for item_key in pending_items.0.drain(..) {
             if let Some(def) = item_db.items.get(&item_key) {
                 let size = ItemSize { width: def.width as i32, height: def.height as i32 };
                 if let Some(pos) = grid_state.find_free_spot(size) {
                     spawn_item_entity(
                         &mut commands,
                         container,
                         def,
                         pos,
                         size,
                         &mut grid_state
                     );
                 }
            }
        }
    }
}

fn spawn_item_entity(
    commands: &mut Commands,
    container: Entity,
    def: &ItemDefinition,
    pos: IVec2,
    size: ItemSize,
    grid_state: &mut InventoryGridState,
) {
     let left = 10.0 + pos.x as f32 * 52.0;
     let top = 10.0 + pos.y as f32 * 52.0;
     let width = size.width as f32 * 50.0 + (size.width - 1) as f32 * 2.0;
     let height = size.height as f32 * 50.0 + (size.height - 1) as f32 * 2.0;

     let item_entity = commands.spawn((
        Node {
            width: Val::Px(width),
            height: Val::Px(height),
            position_type: PositionType::Absolute,
            left: Val::Px(left),
            top: Val::Px(top),
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.5, 0.5, 0.8)),
        BorderColor(Color::WHITE),
        Interaction::default(),
        Item,
        GridPosition { x: pos.x, y: pos.y },
        size,
        def.clone(),
    ))
    .with_children(|parent| {
         parent.spawn((
             Text::new(&def.name),
             TextFont { font_size: 14.0, ..default() },
             TextColor(Color::WHITE),
             Node {
                 position_type: PositionType::Absolute,
                 left: Val::Px(2.0),
                 top: Val::Px(2.0),
                 ..default()
             },
             PickingBehavior::IGNORE,
         ));
    })
    .observe(handle_drag_start)
    .observe(handle_drag)
    .observe(handle_drag_drop)
    .observe(handle_drag_end)
    .id();

    for dy in 0..size.height {
        for dx in 0..size.width {
            grid_state.cells.insert(IVec2::new(pos.x + dx, pos.y + dy), item_entity);
        }
    }
    commands.entity(container).add_child(item_entity);
}

fn debug_spawn_item_system(
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    mut grid_state: ResMut<InventoryGridState>,
    item_db: Res<ItemDatabase>,
    q_container: Query<Entity, With<InventoryGridContainer>>,
) {
    if input.just_pressed(KeyCode::KeyG) {
        if let Ok(container) = q_container.get_single() {
            let mut rng = rand::thread_rng();
            let keys: Vec<&String> = item_db.items.keys().collect();
            if keys.is_empty() { return; }
            let random_key = keys[rng.gen_range(0..keys.len())];

            if let Some(def) = item_db.items.get(random_key) {
                 let size = ItemSize { width: def.width as i32, height: def.height as i32 };
                 if let Some(pos) = grid_state.find_free_spot(size) {
                     spawn_item_entity(
                         &mut commands,
                         container,
                         def,
                         pos,
                         size,
                         &mut grid_state
                     );
                 }
            }
        }
    }
}

fn attach_drag_observers(
    trigger: Trigger<ItemSpawnedEvent>,
    mut commands: Commands,
) {
    let entity = trigger.event().0;
    commands.entity(entity)
        .observe(handle_drag_start)
        .observe(handle_drag)
        .observe(handle_drag_drop)
        .observe(handle_drag_end);
}

// ====================
// Updated Drag Handlers
// ====================

fn handle_drag_start(
    trigger: Trigger<Pointer<DragStart>>,
    mut commands: Commands,
    mut q_node: Query<(&mut ZIndex, &Node, &Parent)>,
) {
    let entity = trigger.entity();
    if let Ok((mut z_index, node, parent)) = q_node.get_mut(entity) {
        commands.entity(entity).insert(DragOriginalPosition {
            parent: parent.get(),
            left: node.left,
            top: node.top,
            z_index: *z_index,
        });

        *z_index = ZIndex(100);

        commands.entity(entity).insert(PickingBehavior {
            should_block_lower: false,
            ..default()
        });
    }
}

fn handle_drag(
    trigger: Trigger<Pointer<Drag>>,
    mut q_node: Query<&mut Node>,
) {
    let entity = trigger.entity();
    if let Ok(mut node) = q_node.get_mut(entity) {
        let event = trigger.event();
        if let Val::Px(current_left) = node.left {
            node.left = Val::Px(current_left + event.delta.x);
        } else {
             // Handle case where item started with no top/left set (flex layout)
             if let Val::Auto = node.left { node.left = Val::Px(event.delta.x); }
             if let Val::Auto = node.top { node.top = Val::Px(event.delta.y); }
        }

        if let Val::Px(current_top) = node.top {
            node.top = Val::Px(current_top + event.delta.y);
        } else {
             if let Val::Auto = node.left { node.left = Val::Px(event.delta.x); }
             if let Val::Auto = node.top { node.top = Val::Px(event.delta.y); }
        }
    }
}

fn handle_drag_end(
    trigger: Trigger<Pointer<DragEnd>>,
    mut commands: Commands,
) {
    let entity = trigger.entity();
    commands.entity(entity).remove::<PickingBehavior>();
}

fn handle_drag_drop(
    trigger: Trigger<Pointer<DragDrop>>,
    mut commands: Commands,
    mut q_item: Query<(Entity, &mut ZIndex, &mut Node, &ItemSize, Option<&mut GridPosition>, Option<&ShopItem>, &ItemDefinition)>,
    q_original: Query<&DragOriginalPosition>,
    q_global_transform: Query<&GlobalTransform>,
    mut grid_state: ResMut<InventoryGridState>,
    q_grid_container: Query<(Entity, &GlobalTransform), With<InventoryGridContainer>>,
    q_shop_container: Query<(Entity, &GlobalTransform), With<ShopContainer>>,
    mut player_stats: ResMut<PlayerStats>,
    mut shop_state: ResMut<ShopState>,
) {
    let entity = trigger.entity();

    // We need to resolve where we dropped relative to the Grid Container.
    // Using GlobalTransform to map coordinates.

    if let Ok((entity, mut z_index, mut node, size, mut grid_pos_opt, shop_item_opt, def)) = q_item.get_mut(entity) {

        // 1. Get Drop Position in World Space
        let item_transform = q_global_transform.get(entity).unwrap();
        let drop_pos = item_transform.translation(); // This is center of the item usually

        // ==========================
        // SHOP SELL CHECK
        // ==========================
        if let Ok((_shop_entity, shop_transform)) = q_shop_container.get_single() {
             let delta = drop_pos - shop_transform.translation();
             if delta.x.abs() < 150.0 && delta.y.abs() < 200.0 {
                 // Check if it's an owned item
                 if let Some(mut grid_pos) = grid_pos_opt { // Consumes grid_pos_opt here
                     // SELL
                     // Refund 50%
                     let refund = def.cost / 2;
                     player_stats.thalers += refund;

                     // Remove from Grid State
                      for dy in 0..size.height {
                             for dx in 0..size.width {
                                 let old_pos = IVec2::new(grid_pos.x + dx, grid_pos.y + dy);
                                 if let Some(occupier) = grid_state.cells.get(&old_pos) {
                                     if *occupier == entity {
                                         grid_state.cells.remove(&old_pos);
                                     }
                                 }
                             }
                         }

                     // Despawn entity
                     commands.entity(entity).despawn_recursive();
                     return;
                 }
             }
        }

        // ==========================
        // GRID PLACEMENT CHECK
        // ==========================
        if let Ok((grid_entity, grid_transform)) = q_grid_container.get_single() {
            let delta = drop_pos - grid_transform.translation();

            let grid_pixel_width = grid_state.width as f32 * 52.0 + 20.0;
            let grid_pixel_height = grid_state.height as f32 * 52.0 + 20.0;

            let item_w = size.width as f32 * 50.0 + (size.width - 1) as f32 * 2.0;
            let item_h = size.height as f32 * 50.0 + (size.height - 1) as f32 * 2.0;

            let calc_left = delta.x + grid_pixel_width / 2.0 - item_w / 2.0;
            let calc_top = (grid_pixel_height / 2.0 - delta.y) - item_h / 2.0;

            let padding = 10.0;
            let stride = 52.0;

            let target_x = ((calc_left - padding) / stride).round() as i32;
            let target_y = ((calc_top - padding) / stride).round() as i32;

            // Check bounds
            if target_x >= 0 && target_y >= 0 && target_x < grid_state.width && target_y < grid_state.height {
                 let target_pos = IVec2::new(target_x, target_y);

                 // Case A: Dragging SHOP ITEM -> GRID
                 if let Some(shop_item) = shop_item_opt {
                     if grid_state.is_area_free(target_pos, *size, None) {
                         if player_stats.thalers >= def.cost {
                             // BUY!
                             player_stats.thalers -= def.cost;

                             commands.entity(entity).remove::<ShopItem>();
                             commands.entity(entity).insert(Item);
                             commands.entity(entity).insert(GridPosition { x: target_x, y: target_y });
                             commands.entity(entity).remove::<DragOriginalPosition>();

                             commands.entity(entity).set_parent(grid_entity);

                             let new_left = padding + target_x as f32 * stride;
                             let new_top = padding + target_y as f32 * stride;
                             node.position_type = PositionType::Absolute;
                             node.left = Val::Px(new_left);
                             node.top = Val::Px(new_top);
                             node.margin = UiRect::all(Val::Px(0.0));

                             commands.entity(entity).insert(BackgroundColor(Color::srgb(0.5, 0.5, 0.8)));
                             commands.entity(entity).insert(BorderColor(Color::WHITE));

                             for dy in 0..size.height {
                                 for dx in 0..size.width {
                                     grid_state.cells.insert(IVec2::new(target_x + dx, target_y + dy), entity);
                                 }
                             }

                             shop_state.items[shop_item.slot_index] = None;

                             return;
                         }
                     }
                 }
                 // Case B: Dragging OWNED ITEM -> GRID
                 else if let Some(mut grid_pos) = grid_pos_opt {
                      // Check validity (excluding self)
                      if grid_state.is_area_free(target_pos, *size, Some(entity)) {
                           // Clear old
                           for dy in 0..size.height {
                                 for dx in 0..size.width {
                                     let old_pos = IVec2::new(grid_pos.x + dx, grid_pos.y + dy);
                                     if let Some(occupier) = grid_state.cells.get(&old_pos) {
                                         if *occupier == entity {
                                             grid_state.cells.remove(&old_pos);
                                         }
                                     }
                                 }
                             }
                             // Set new
                             for dy in 0..size.height {
                                 for dx in 0..size.width {
                                     let new_pos = IVec2::new(target_x + dx, target_y + dy);
                                     grid_state.cells.insert(new_pos, entity);
                                 }
                             }

                             let new_left = padding + target_x as f32 * stride;
                             let new_top = padding + target_y as f32 * stride;
                             node.left = Val::Px(new_left);
                             node.top = Val::Px(new_top);
                             grid_pos.x = target_x;
                             grid_pos.y = target_y;

                             commands.entity(entity).remove::<DragOriginalPosition>();
                             if let Ok(original) = q_original.get(entity) {
                                 *z_index = original.z_index;
                             }
                             return;
                      }
                 }
            }
        }
    }

    // Revert if failed
    if let Ok(original) = q_original.get(entity) {
        if let Ok((_, mut z_index, mut node, _, _, _, _)) = q_item.get_mut(entity) {
             *z_index = original.z_index;
             node.left = original.left;
             node.top = original.top;
        }
        commands.entity(entity).remove::<DragOriginalPosition>();
    }
}
