use bevy::prelude::*;
use bevy::utils::HashMap;
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
pub struct ItemRotation {
    pub value: u8, // 0..3
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
    pub rotation: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CellState {
    Free,
    Occupied(Entity),
}

#[derive(Clone, Debug)]
pub struct Cell {
    pub state: CellState,
}

// Resources
#[derive(Resource)]
pub struct InventoryGridState {
   pub grid: HashMap<IVec2, Cell>,
   pub width: i32,
   pub height: i32,
}

impl Default for InventoryGridState {
    fn default() -> Self {
        let mut grid = HashMap::new();
        // Initialize a default "backpack" shape (e.g., 6x4 in the middle)
        for y in 2..6 {
            for x in 1..7 {
                grid.insert(IVec2::new(x, y), Cell { state: CellState::Free });
            }
        }

        Self {
            grid,
            width: 8,
            height: 8,
        }
    }
}

impl InventoryGridState {
    // Helper to rotate a shape
    pub fn get_rotated_shape(shape: &Vec<IVec2>, rotation_step: u8) -> Vec<IVec2> {
        let steps = rotation_step % 4;
        if steps == 0 {
            return shape.clone();
        }

        shape.iter().map(|point| {
            let mut p = *point;
            for _ in 0..steps {
                // Rotate 90 degrees clockwise: (x, y) -> (-y, x)
                let old_x = p.x;
                let old_y = p.y;
                p.x = -old_y;
                p.y = old_x;
            }
            p
        }).collect()
    }

    // Helper to get bounding box info
    // Returns (min_x, min_y, width_slots, height_slots)
    pub fn calculate_bounding_box(shape: &Vec<IVec2>, rotation_step: u8) -> (i32, i32, i32, i32) {
        let rotated_shape = Self::get_rotated_shape(shape, rotation_step);
        if rotated_shape.is_empty() {
            return (0, 0, 1, 1);
        }

        let mut min_x = rotated_shape[0].x;
        let mut max_x = rotated_shape[0].x;
        let mut min_y = rotated_shape[0].y;
        let mut max_y = rotated_shape[0].y;

        for p in &rotated_shape {
            if p.x < min_x { min_x = p.x; }
            if p.x > max_x { max_x = p.x; }
            if p.y < min_y { min_y = p.y; }
            if p.y > max_y { max_y = p.y; }
        }

        (min_x, min_y, max_x - min_x + 1, max_y - min_y + 1)
    }

    // New validation function
    pub fn can_place_item(&self, item_shape: &Vec<IVec2>, pos: IVec2, rotation_step: u8, exclude_entity: Option<Entity>) -> bool {
        let rotated_shape = Self::get_rotated_shape(item_shape, rotation_step);

        for offset in rotated_shape {
            let target_pos = pos + offset;

            // Check if cell exists (is valid slot)
            match self.grid.get(&target_pos) {
                Some(cell) => {
                    // Check if occupied
                    if let CellState::Occupied(occupier) = cell.state {
                         if Some(occupier) != exclude_entity {
                             return false;
                         }
                    }
                },
                None => return false, // Out of bounds / invalid slot
            }
        }
        true
    }

    // Kept for compatibility with existing systems (mostly debug/random spawn), updated to use shape
    pub fn find_free_spot(&self, def: &ItemDefinition) -> Option<IVec2> {
        for y in 0..self.height {
            for x in 0..self.width {
                let pos = IVec2::new(x, y);
                if self.can_place_item(&def.shape, pos, 0, None) {
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
    for cell in grid_state.grid.values_mut() {
        cell.state = CellState::Free;
    }

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
                       let pos = IVec2::new(x, y);
                       let is_valid = grid_state.grid.contains_key(&pos);
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
                 let pos = IVec2::new(saved_item.grid_x, saved_item.grid_y);

                 if grid_state.can_place_item(&def.shape, pos, 0, None) {
                     spawn_item_entity(
                         &mut commands,
                         container,
                         def,
                         pos,
                         0, // Default rotation 0 for now
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

                 // Find free spot
                 if let Some(pos) = grid_state.find_free_spot(def) {
                     spawn_item_entity(
                         &mut commands,
                         container,
                         def,
                         pos,
                         0,
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
    rotation: u8,
    grid_state: &mut InventoryGridState,
) {
     let (min_x, min_y, width_slots, height_slots) = InventoryGridState::calculate_bounding_box(&def.shape, rotation);

     // Size for UI
     let width_px = width_slots as f32 * 50.0 + (width_slots - 1) as f32 * 2.0;
     let height_px = height_slots as f32 * 50.0 + (height_slots - 1) as f32 * 2.0;

     let effective_x = pos.x + min_x;
     let effective_y = pos.y + min_y;

     let left = 10.0 + effective_x as f32 * 52.0;
     let top = 10.0 + effective_y as f32 * 52.0;

     let item_entity = commands.spawn((
        Node {
            width: Val::Px(width_px),
            height: Val::Px(height_px),
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
        ItemSize { width: width_slots, height: height_slots },
        ItemRotation { value: rotation },
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
             PickingBehavior::IGNORE,
         ));
    })
    .observe(handle_drag_start)
    .observe(handle_drag)
    .observe(handle_drag_drop)
    .observe(handle_drag_end)
    .id();

    // Add to grid state
    let rotated_shape = InventoryGridState::get_rotated_shape(&def.shape, rotation);
    for offset in rotated_shape {
        let cell_pos = pos + offset;
        if let Some(cell) = grid_state.grid.get_mut(&cell_pos) {
            cell.state = CellState::Occupied(item_entity);
        }
    }

    commands.entity(container).add_child(item_entity);
}

fn rotate_item_input_system(
    input: Res<ButtonInput<KeyCode>>,
    mut q_dragged_item: Query<(Entity, &mut ItemRotation, &mut ItemSize, &mut Node, &ItemDefinition), With<DragOriginalPosition>>,
) {
    if input.just_pressed(KeyCode::KeyR) {
        for (_entity, mut rot, mut size, mut node, def) in q_dragged_item.iter_mut() {
            // Update rotation
            rot.value = (rot.value + 1) % 4;

            let (_min_x, _min_y, width_slots, height_slots) = InventoryGridState::calculate_bounding_box(&def.shape, rot.value);

             size.width = width_slots;
             size.height = height_slots;

             // Update Node size
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
            let mut rng = rand::thread_rng();
            let keys: Vec<&String> = item_db.items.keys().collect();
            if keys.is_empty() { return; }
            let random_key = keys[rng.gen_range(0..keys.len())];

            if let Some(def) = item_db.items.get(random_key) {
                 if let Some(pos) = grid_state.find_free_spot(def) {
                     spawn_item_entity(
                         &mut commands,
                         container,
                         def,
                         pos,
                         0,
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
    mut q_node: Query<(&mut ZIndex, &Node, &ItemRotation)>,
) {
    let entity = trigger.entity();
    if let Ok((mut z_index, node, rotation)) = q_node.get_mut(entity) {
        commands.entity(entity).insert(DragOriginalPosition {
            left: node.left,
            top: node.top,
            z_index: *z_index,
            rotation: rotation.value,
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
        }
        if let Val::Px(current_top) = node.top {
            node.top = Val::Px(current_top + event.delta.y);
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
    mut q_item: Query<(&mut ZIndex, &mut Node, &mut ItemRotation, &mut ItemSize, &mut GridPosition, &ItemDefinition), With<Item>>,
    q_original: Query<&DragOriginalPosition>,
    mut grid_state: ResMut<InventoryGridState>,
) {
    let entity = trigger.entity();

    if let Ok((mut z_index, mut node, mut rotation, mut size, mut grid_pos, def)) = q_item.get_mut(entity) {
        let mut left_val = 0.0;
        let mut top_val = 0.0;

        if let Val::Px(l) = node.left { left_val = l; }
        if let Val::Px(t) = node.top { top_val = t; }

        let padding = 10.0;
        let stride = 52.0;

        // Visual TopLeft of the Node
        // We need to determine the Grid Pivot (x,y).
        let (min_x, min_y, _, _) = InventoryGridState::calculate_bounding_box(&def.shape, rotation.value);

        let estimated_pivot_x = ((left_val - padding) / stride).round() as i32 - min_x;
        let estimated_pivot_y = ((top_val - padding) / stride).round() as i32 - min_y;

        let target_pos = IVec2::new(estimated_pivot_x, estimated_pivot_y);

        // Validation
        if grid_state.can_place_item(&def.shape, target_pos, rotation.value, Some(entity)) {
             // Clear old grid positions
             let mut cells_to_clear = Vec::new();
             for (pos, cell) in grid_state.grid.iter() {
                 if let CellState::Occupied(occupier) = cell.state {
                     if occupier == entity {
                         cells_to_clear.push(*pos);
                     }
                 }
             }
             for pos in cells_to_clear {
                 if let Some(cell) = grid_state.grid.get_mut(&pos) {
                     cell.state = CellState::Free;
                 }
             }

             // Occupy new positions
             let rotated_shape = InventoryGridState::get_rotated_shape(&def.shape, rotation.value);
             for offset in rotated_shape {
                 let cell_pos = target_pos + offset;
                 if let Some(cell) = grid_state.grid.get_mut(&cell_pos) {
                     cell.state = CellState::Occupied(entity);
                 }
             }

             // Snap to exact slot position
             let effective_x = target_pos.x + min_x;
             let effective_y = target_pos.y + min_y;

             let new_left = padding + effective_x as f32 * stride;
             let new_top = padding + effective_y as f32 * stride;

             node.left = Val::Px(new_left);
             node.top = Val::Px(new_top);

             // Update logical position
             grid_pos.x = target_pos.x;
             grid_pos.y = target_pos.y;

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

    // Revert
    if let Ok(original) = q_original.get(entity) {
        if let Ok((mut z_index, mut node, mut rotation, mut size, _, def)) = q_item.get_mut(entity) {
             *z_index = original.z_index;
             node.left = original.left;
             node.top = original.top;

             // Restore rotation
             if rotation.value != original.rotation {
                 rotation.value = original.rotation;
                 // Restore Size/Visuals
                 let (_min_x, _min_y, width_slots, height_slots) = InventoryGridState::calculate_bounding_box(&def.shape, rotation.value);
                 size.width = width_slots;
                 size.height = height_slots;

                 let width_px = size.width as f32 * 50.0 + (size.width - 1) as f32 * 2.0;
                 let height_px = size.height as f32 * 50.0 + (size.height - 1) as f32 * 2.0;
                 node.width = Val::Px(width_px);
                 node.height = Val::Px(height_px);
             }
        }
        commands.entity(entity).remove::<DragOriginalPosition>();
    }
}
