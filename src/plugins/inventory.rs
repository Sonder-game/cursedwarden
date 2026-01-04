use bevy::prelude::*;
use bevy::utils::{HashMap, HashSet};
use crate::plugins::core::GameState;
use crate::plugins::items::{ItemDatabase, ItemDefinition};
use crate::plugins::metagame::{PersistentInventory, SavedItem};
use rand::Rng;

pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InventoryGridState>()
           .add_systems(OnEnter(GameState::EveningPhase), (spawn_inventory_ui, apply_deferred, load_inventory_state, apply_deferred, consume_pending_items).chain())
           .add_systems(OnExit(GameState::EveningPhase), (save_inventory_state, cleanup_inventory_ui).chain())
           .add_systems(Update, (resize_item_system, debug_spawn_item_system, rotate_item_input_system).run_if(in_state(GameState::EveningPhase)))
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
pub struct Item;

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

#[derive(Component, Default)]
pub struct DragOriginalPosition {
    pub left: Val,
    pub top: Val,
    pub z_index: ZIndex,
}

// Resources
#[derive(Resource)]
pub struct InventoryGridState {
   pub cells: HashMap<IVec2, Entity>,
   pub valid_cells: HashSet<IVec2>,
   pub width: i32,
   pub height: i32,
}

impl Default for InventoryGridState {
    fn default() -> Self {
        let mut valid_cells = HashSet::new();
        // Initialize a default "backpack" shape (e.g., 6x4 in the middle)
        // Center it in the 8x8 grid for now (offsets 1, 2)
        for y in 2..6 {
            for x in 1..7 {
                valid_cells.insert(IVec2::new(x, y));
            }
        }

        Self {
            cells: HashMap::new(),
            valid_cells,
            width: 8,
            height: 8,
        }
    }
}

impl InventoryGridState {
    pub fn is_area_free(&self, pos: IVec2, size: ItemSize, exclude_entity: Option<Entity>) -> bool {
        // Check bounds (overall grid size)
        if pos.x < 0 || pos.y < 0 || pos.x + size.width > self.width || pos.y + size.height > self.height {
            return false;
        }

        // Check collisions and validity
        for dy in 0..size.height {
            for dx in 0..size.width {
                let check_pos = IVec2::new(pos.x + dx, pos.y + dy);

                // Check if cell is a valid inventory slot
                if !self.valid_cells.contains(&check_pos) {
                    return false;
                }

                // Check if occupied
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
    // Clear the grid state map, as we are rebuilding the UI and the entities within it.
    // However, if we are loading from persistent state, we need it empty anyway.
    grid_state.cells.clear();

    // Root Node
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                display: Display::Flex,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
            InventoryUiRoot,
        ))
        .with_children(|parent| {
            // Inventory Grid Container
            parent.spawn((
                InventoryGridContainer,
                Node {
                    display: Display::Grid,
                    grid_template_columns: vec![GridTrack::px(50.0); grid_state.width as usize],
                    grid_template_rows: vec![GridTrack::px(50.0); grid_state.height as usize],
                    row_gap: Val::Px(2.0),
                    column_gap: Val::Px(2.0),
                    padding: UiRect::all(Val::Px(10.0)),
                    // Ensure relative positioning context for children (items)
                    position_type: PositionType::Relative,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
            ))
            .with_children(|grid_parent| {
                // Spawn Slots
                for y in 0..grid_state.height {
                    for x in 0..grid_state.width {
                       let is_valid = grid_state.valid_cells.contains(&IVec2::new(x, y));
                       let bg_color = if is_valid {
                           Color::srgb(0.3, 0.3, 0.3)
                       } else {
                           Color::srgba(0.1, 0.1, 0.1, 0.5) // Darker/Transparent for invalid
                       };

                       let border_color = if is_valid {
                            Color::BLACK
                       } else {
                            Color::srgba(0.0, 0.0, 0.0, 0.2)
                       };

                       grid_parent.spawn((
                            Node {
                                width: Val::Px(50.0),
                                height: Val::Px(50.0),
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(bg_color),
                            BorderColor(border_color),
                            InventorySlot { x, y },
                        ));
                    }
                }
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

                 // Check if space is actually free (sanity check)
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

                 // Find free spot
                 if let Some(pos) = grid_state.find_free_spot(size) {
                     spawn_item_entity(
                         &mut commands,
                         container,
                         def,
                         pos,
                         size,
                         &mut grid_state
                     );
                     info!("Consumed pending item {} at {:?}", def.name, pos);
                 } else {
                     warn!("No space for pending item {}", def.name);
                 }
            } else {
                warn!("Unknown item id: {}", item_key);
            }
        }
    } else {
        warn!("Grid container not found during consume_pending_items");
    }
}

// Helper to spawn item and attach to grid
fn spawn_item_entity(
    commands: &mut Commands,
    container: Entity,
    def: &ItemDefinition,
    pos: IVec2,
    size: ItemSize,
    grid_state: &mut InventoryGridState,
) {
     // Calculate UI position
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
             TextFont {
                 font_size: 14.0,
                 ..default()
             },
             TextColor(Color::WHITE),
             Node {
                 position_type: PositionType::Absolute,
                 left: Val::Px(2.0),
                 top: Val::Px(2.0),
                 ..default()
             },
             PickingBehavior::IGNORE, // Text shouldn't block drag
         ));
    })
    .observe(handle_drag_start)
    .observe(handle_drag)
    .observe(handle_drag_drop)
    .observe(handle_drag_end) // Added DragEnd handler
    .id();

    // Add to grid state
    for dy in 0..size.height {
        for dx in 0..size.width {
            grid_state.cells.insert(IVec2::new(pos.x + dx, pos.y + dy), item_entity);
        }
    }

    commands.entity(container).add_child(item_entity);
}

fn rotate_item_input_system(
    input: Res<ButtonInput<KeyCode>>,
    mut q_dragged_item: Query<(Entity, &mut ItemSize, &mut Node), With<DragOriginalPosition>>,
) {
    if input.just_pressed(KeyCode::KeyR) {
        for (_entity, mut size, mut node) in q_dragged_item.iter_mut() {
            // Swap width and height
            let new_width = size.height;
            let new_height = size.width;
            size.width = new_width;
            size.height = new_height;

            // Update Node size immediately for visual feedback
            // (resize_item_system handles it too, but this makes it snappy)
            let width_px = size.width as f32 * 50.0 + (size.width - 1) as f32 * 2.0;
            let height_px = size.height as f32 * 50.0 + (size.height - 1) as f32 * 2.0;
            node.width = Val::Px(width_px);
            node.height = Val::Px(height_px);
        }
    }
}

fn debug_spawn_item_system(
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    mut grid_state: ResMut<InventoryGridState>,
    item_db: Res<ItemDatabase>,
    q_container: Query<Entity, With<InventoryGridContainer>>,
) {
    if input.just_pressed(KeyCode::Space) {
        if let Ok(container) = q_container.get_single() {
            // Pick a random item
            let mut rng = rand::thread_rng();
            let keys: Vec<&String> = item_db.items.keys().collect();
            if keys.is_empty() { return; }
            let random_key = keys[rng.gen_range(0..keys.len())];

            if let Some(def) = item_db.items.get(random_key) {
                 let size = ItemSize { width: def.width as i32, height: def.height as i32 };

                 // Find free spot
                 if let Some(pos) = grid_state.find_free_spot(size) {
                     spawn_item_entity(
                         &mut commands,
                         container,
                         def,
                         pos,
                         size,
                         &mut grid_state
                     );
                     info!("Spawned item {} at {:?}", def.name, pos);
                 } else {
                     warn!("No space for item {}", def.name);
                 }
            }
        } else {
            warn!("Grid container not found");
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

// Drag Handlers
fn handle_drag_start(
    trigger: Trigger<Pointer<DragStart>>,
    mut commands: Commands,
    mut q_node: Query<(&mut ZIndex, &Node)>,
) {
    let entity = trigger.entity();
    if let Ok((mut z_index, node)) = q_node.get_mut(entity) {
        // Store original position for potential revert
        commands.entity(entity).insert(DragOriginalPosition {
            left: node.left,
            top: node.top,
            z_index: *z_index,
        });

        // Bring to front (ZIndex(100))
        *z_index = ZIndex(100);

        // Allow raycast to pass through to find drop target (Slot)
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
        // Update position based on delta
        if let Val::Px(current_left) = node.left {
            node.left = Val::Px(current_left + event.delta.x);
        }
        if let Val::Px(current_top) = node.top {
            node.top = Val::Px(current_top + event.delta.y);
        }
    }
}

// Ensure picking behavior is reset if drag ends (e.g. cancelled or dropped)
fn handle_drag_end(
    trigger: Trigger<Pointer<DragEnd>>,
    mut commands: Commands,
) {
    let entity = trigger.entity();
    // Re-enable blocking so we can drag it again later
    commands.entity(entity).remove::<PickingBehavior>();
}

fn handle_drag_drop(
    trigger: Trigger<Pointer<DragDrop>>,
    mut commands: Commands,
    mut q_item: Query<(&mut ZIndex, &mut Node, &ItemSize, &mut GridPosition), With<Item>>,
    q_original: Query<&DragOriginalPosition>,
    mut grid_state: ResMut<InventoryGridState>,
) {
    let entity = trigger.entity();

    if let Ok((mut z_index, mut node, size, mut grid_pos)) = q_item.get_mut(entity) {
        let mut left_val = 0.0;
        let mut top_val = 0.0;

        if let Val::Px(l) = node.left { left_val = l; }
        if let Val::Px(t) = node.top { top_val = t; }

        // Grid parameters (must match spawn_inventory_ui)
        let padding = 10.0;
        let slot_size = 50.0;
        let gap = 2.0;
        let stride = slot_size + gap; // 52.0

        // Calculate closest grid index based on top-left corner
        // Using round() to snap to the nearest slot center effectively
        let target_x = ((left_val - padding) / stride).round() as i32;
        let target_y = ((top_val - padding) / stride).round() as i32;

        let target_pos = IVec2::new(target_x, target_y);

        // Basic validation and collision check
        if grid_state.is_area_free(target_pos, *size, Some(entity)) {
             // Clear old grid positions
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

             // Set new grid positions
             for dy in 0..size.height {
                 for dx in 0..size.width {
                     let new_pos = IVec2::new(target_x + dx, target_y + dy);
                     grid_state.cells.insert(new_pos, entity);
                 }
             }

             // Snap to exact slot position
             let new_left = padding + target_x as f32 * stride;
             let new_top = padding + target_y as f32 * stride;

             node.left = Val::Px(new_left);
             node.top = Val::Px(new_top);

             // Update logical position
             grid_pos.x = target_x;
             grid_pos.y = target_y;

             // Restore Z-Index
             if let Ok(original) = q_original.get(entity) {
                  *z_index = original.z_index;
             } else {
                  *z_index = ZIndex(0);
             }

             commands.entity(entity).remove::<DragOriginalPosition>();
             return;
        }
    }

    // If invalid or out of bounds, revert
    if let Ok(original) = q_original.get(entity) {
        if let Ok((mut z_index, mut node, _, _)) = q_item.get_mut(entity) {
             *z_index = original.z_index;
             node.left = original.left;
             node.top = original.top;
        }
        commands.entity(entity).remove::<DragOriginalPosition>();
    }
}
