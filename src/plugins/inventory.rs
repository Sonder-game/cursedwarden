use bevy::prelude::*;
use bevy::utils::HashMap;
use crate::plugins::core::GameState;
use crate::plugins::items::{ItemDefinition, ItemType};

/// Plugin managing all inventory logic, grid, and interaction.
/// Implements "Inventory Tetris" mechanics using Bevy Observers.
pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
   fn build(&self, app: &mut App) {
       app
           // Resources: Single source of truth for grid topology
          .init_resource::<InventoryGridState>()
          .init_resource::<InteractionState>()
           // Events: Signal changes for stat recalculation
          .add_event::<InventoryChangedEvent>()
           // UI Lifecycle Systems
          .add_systems(OnEnter(GameState::EveningPhase), setup_inventory_ui)
          .add_systems(OnExit(GameState::EveningPhase), cleanup_inventory)
           // Update Systems (run only in inventory phase)
          .add_systems(
               Update,
               (
                   update_drag_visuals,        // Visual validation (red/green)
                   update_item_transforms,     // Smooth snapping
               ).run_if(in_state(GameState::EveningPhase))
           )
           // Bevy Picking Observers: New event system for Drag & Drop (Bevy 0.15)
          .add_observer(on_drag_start)
          .add_observer(on_drag)
          .add_observer(on_drag_end);
   }
}

// ============================================================================
// CONSTANTS AND SETTINGS
// ============================================================================

pub const CELL_SIZE: f32 = 64.0;
pub const CELL_GAP: f32 = 2.0;
// Effective grid step for calculations (Size + Gap)
pub const GRID_STEP: f32 = CELL_SIZE;

// ============================================================================
// COMPONENTS
// ============================================================================

/// Main item component. Stores its ID and base shape.
#[derive(Component)]
pub struct InventoryItem {
   pub item_id: String,
   /// Shape defined by list of offsets from (0,0). Example: [(0,0), (1,0), (0,1)]
   pub base_shape: Vec<IVec2>,
   pub width: u8,
   pub height: u8,
}

/// Bag component. A bag is an item that PROVIDES slots.
#[derive(Component)]
pub struct Bag {
   pub provided_slots: Vec<IVec2>,
}

/// Logical grid position (X, Y).
/// (0,0) corresponds to the top-left corner of the grid container.
#[derive(Component, Debug, Clone, Copy)]
pub struct GridPosition(pub IVec2);

/// Current rotation: 0=0°, 1=90°, 2=180°, 3=270°.
#[derive(Component, Debug, Clone, Copy)]
pub struct ItemRotation(pub u8);

/// Marker for items in "Storage" (Limbo), not on the grid.
#[derive(Component)]
pub struct InStorage;

/// Marker for the inventory UI root node.
#[derive(Component)]
pub struct InventoryUiRoot;

/// Marker for the container representing the visual grid.
#[derive(Component)]
pub struct InventoryGridContainer;

// ============================================================================
// RESOURCES
// ============================================================================

/// Global grid state. Used for fast collision checks (O(1)).
#[derive(Resource, Default)]
pub struct InventoryGridState {
   /// Occupancy map: Coordinate -> Item Entity
   pub occupancy: HashMap<IVec2, Entity>,
   /// Slot map: Coordinate -> Bag Entity providing the slot
   pub slots: HashMap<IVec2, Entity>,
   /// Active zone bounds (to limit bag movement).
   pub bounds: IRect,
}

/// State of the current drag operation.
#[derive(Resource, Default)]
pub struct InteractionState {
   pub dragged_entity: Option<Entity>,
   /// Original position (for revert on invalid drop)
   pub original_grid_pos: IVec2,
   pub original_rotation: u8,
   pub was_in_storage: bool,
}

#[derive(Event)]
pub struct InventoryChangedEvent;

// ============================================================================
// GRID ALGORITHMS CORE
// ============================================================================

impl InventoryGridState {
   /// Full rebuild of slot and occupancy maps.
   /// Called after any successful inventory change.
   pub fn rebuild(
       &mut self,
       bags: &Query<(Entity, &GridPosition, &ItemRotation, &Bag)>,
       items: &Query<(Entity, &GridPosition, &ItemRotation, &InventoryItem), Without<Bag>>,
   ) {
       self.slots.clear();
       self.occupancy.clear();
       self.bounds = IRect::new(0, 0, 0, 0);

       // 1. Project Bags onto grid (Create "Background" of slots)
       for (entity, pos, rot, bag) in bags.iter() {
           let shape = rotate_shape(&bag.provided_slots, rot.0);
           for offset in shape {
               let slot_pos = pos.0 + offset;
               // If slots overlap, last one wins (or logic to forbid could be added)
               self.slots.insert(slot_pos, entity);

               // Expand bounds
               self.bounds.min = self.bounds.min.min(slot_pos);
               self.bounds.max = self.bounds.max.max(slot_pos);
           }
       }

       // 2. Place Items (Fill "Foreground")
       for (entity, pos, rot, item) in items.iter() {
           // Items in storage are ignored (query filter should handle this, but adding check)
           let shape = rotate_shape(&item.base_shape, rot.0);
           for offset in shape {
               let cell = pos.0 + offset;

               // Collision check during rebuild (for debug)
               if self.occupancy.contains_key(&cell) {
                   warn!("Collision detected during rebuild at {:?}! Entity {:?}", cell, entity);
               }
               self.occupancy.insert(cell, entity);
           }
       }
   }

   /// Checks if an ITEM can be placed at given coordinates.
   /// Core "Tetris" logic.
   pub fn can_place_item(
       &self,
       shape: &[IVec2],
       pos: IVec2,
       rot: u8,
       ignore_entity: Option<Entity>,
   ) -> bool {
       let rotated = rotate_shape(shape, rot);

       for offset in rotated {
           let target = pos + offset;

           // Rule 1: Must be a valid slot (provided by a bag)
           if !self.slots.contains_key(&target) {
               return false;
           }

           // Rule 2: Slot must not be occupied by another item
           if let Some(occupier) = self.occupancy.get(&target) {
               // If occupied not by us - it's a collision.
               if Some(*occupier) != ignore_entity {
                   return false;
               }
           }
       }
       true
   }

   /// Checks if a BAG can be placed.
   /// Rule: Bags must not overlap each other (in this implementation).
   pub fn can_place_bag(
       &self,
       shape: &[IVec2],
       pos: IVec2,
       rot: u8,
       ignore_entity: Option<Entity>,
   ) -> bool {
       let rotated = rotate_shape(shape, rot);
       for offset in rotated {
           let target = pos + offset;
           // Check if anyone already provides a slot here
           if let Some(provider) = self.slots.get(&target) {
               if Some(*provider) != ignore_entity {
                   return false;
               }
           }
       }
       true
   }

    // Helper: Find a free spot for an item (Basic "First Fit" algorithm)
    // Used by Shop and initial loading
    pub fn find_free_spot(
        &self,
        item_shape: &[IVec2],
        width: u8,
        height: u8,
        preferred_pos: Option<IVec2>,
    ) -> Option<IVec2> {
        // Optimization: iterate within bounds
        let start = self.bounds.min;
        let end = self.bounds.max;

        if let Some(pos) = preferred_pos {
             if self.can_place_item(item_shape, pos, 0, None) {
                 return Some(pos);
             }
        }

        // Brute-force search (could be optimized)
        for y in start.y..=end.y {
            for x in start.x..=end.x {
                let pos = IVec2::new(x, y);
                // Try rotation 0 for now. If needed, loop rotations.
                if self.can_place_item(item_shape, pos, 0, None) {
                    return Some(pos);
                }
            }
        }
        None
    }
}

/// Vector rotation math on discrete grid (90 deg clockwise).
pub fn rotate_shape(shape: &[IVec2], rot: u8) -> Vec<IVec2> {
   let turns = rot % 4;
   if turns == 0 {
       return shape.to_vec();
   }

   shape.iter().map(|p| {
       let mut v = *p;
       for _ in 0..turns {
           // Rotation matrix for screen coords (Y down): (x, y) -> (-y, x)
           v = IVec2::new(-v.y, v.x);
       }
       v
   }).collect()
}

pub use crate::plugins::inventory_utils::calculate_combat_stats;

// ============================================================================
// INTERACTION SYSTEM (OBSERVERS)
// ============================================================================

/// Start drag
fn on_drag_start(
   trigger: Trigger<Pointer<DragStart>>,
   mut commands: Commands,
   q_items: Query<(Entity, &GridPosition, &ItemRotation, Has<InStorage>)>,
   mut interaction: ResMut<InteractionState>,
) {
   let entity = trigger.entity();

   if let Ok((_, grid_pos, rot, in_storage)) = q_items.get(entity) {
       // 1. Save state for potential undo
       interaction.dragged_entity = Some(entity);
       interaction.original_grid_pos = grid_pos.0;
       interaction.original_rotation = rot.0;
       interaction.was_in_storage = in_storage;

       // 2. Visual feedback: Lift item to foreground (Z-Index)
       // Use large local Z-index. GlobalZIndex is better if available.
       commands.entity(entity).insert(ZIndex(100));

       // 3. CRITICAL: Disable Picking for the item itself.
       // This allows the cursor to "see through" the item and detect which container we are over.
       commands.entity(entity).insert(PickingBehavior::IGNORE);
   }
}

/// Drag process (visual update)
fn on_drag(
   trigger: Trigger<Pointer<Drag>>,
   mut q_node: Query<&mut Node>,
) {
   // We update only visual position (Style).
   // Validation logic runs separately in update_drag_visuals.
   let entity = trigger.entity();
   let drag = trigger.event();

   if let Ok(mut node) = q_node.get_mut(entity) {
       if let Val::Px(x) = node.left { node.left = Val::Px(x + drag.delta.x); }
       if let Val::Px(y) = node.top { node.top = Val::Px(y + drag.delta.y); }
   }
}

/// End drag (LMB released)
fn on_drag_end(
   trigger: Trigger<Pointer<DragEnd>>,
   mut commands: Commands,
   // Use ParamSet to resolve borrow conflicts
   mut queries: ParamSet<(
       Query<(Entity, &mut Node, &mut GridPosition, &mut ItemRotation, &InventoryItem, Option<&Bag>, Has<InStorage>)>, // Mutable
       (
           Query<(Entity, &GridPosition, &ItemRotation, &Bag)>, // Bags Read-Only
           Query<(Entity, &GridPosition, &ItemRotation, &InventoryItem), Without<Bag>> // Items Read-Only
       )
   )>,
   mut grid_state: ResMut<InventoryGridState>,
   mut interaction: ResMut<InteractionState>,
   _q_container: Query<(&Node, &GlobalTransform), With<InventoryGridContainer>>,
   mut ev_changed: EventWriter<InventoryChangedEvent>,
) {
   let entity = trigger.entity();

   // Restore interactivity to item
   commands.entity(entity).insert(PickingBehavior::default());
   commands.entity(entity).insert(ZIndex(10)); // Reset Z-index

   let mut placement_success = false;

   {
       let mut q_mutable = queries.p0();
       if let Ok((_, mut node, mut grid_pos, mut rot, item_def, is_bag, _)) = q_mutable.get_mut(entity) {
           // Determine current Node coordinates
           let current_left = if let Val::Px(l) = node.left { l } else { 0.0 };
           let current_top = if let Val::Px(t) = node.top { t } else { 0.0 };

           // Grid Snapping
           // Round to nearest grid integer index
           let grid_x = (current_left / GRID_STEP).round() as i32;
           let grid_y = (current_top / GRID_STEP).round() as i32;
           let target_pos = IVec2::new(grid_x, grid_y);

           // Validation Logic
           // In real game need check if we are over GridContainer
           // For simplicity: if coordinates valid for placement, we are over grid.

           let valid = if is_bag.is_some() {
               grid_state.can_place_bag(&item_def.base_shape, target_pos, rot.0, Some(entity))
           } else {
               grid_state.can_place_item(&item_def.base_shape, target_pos, rot.0, Some(entity))
           };

           if valid {
               // COMMIT: Apply changes
               grid_pos.0 = target_pos;
               commands.entity(entity).remove::<InStorage>();
               placement_success = true;
               ev_changed.send(InventoryChangedEvent);
           } else {
               // REVERT: Rollback to original state
               grid_pos.0 = interaction.original_grid_pos;
               rot.0 = interaction.original_rotation;
               if interaction.was_in_storage {
                   commands.entity(entity).insert(InStorage);
               }
           }
       }
   }

   // If placement successful, need to rebuild grid state
   if placement_success {
       let (bags, items) = queries.p1();
       grid_state.rebuild(&bags, &items);
   }

   // Clear interaction state
   interaction.dragged_entity = None;
}

// ============================================================================
// VISUAL UPDATE SYSTEMS
// ============================================================================

/// Syncs visual Node position with logical GridPosition.
/// Ensures "snapping" after drop and drift correction.
fn update_item_transforms(
   mut q_items: Query<(Entity, &mut Node, &GridPosition), With<InventoryItem>>,
   interaction: Res<InteractionState>,
) {
   for (e, mut node, pos) in q_items.iter_mut() {
       // Skip the item currently being dragged, as its position is controlled by the mouse
       if let Some(dragged) = interaction.dragged_entity {
           if e == dragged {
               continue;
           }
       }

       let target_x = pos.0.x as f32 * GRID_STEP;
       let target_y = pos.0.y as f32 * GRID_STEP;

       // Update only if position differs to avoid unnecessary layout recalc
       // Use small epsilon for float comparison
       if let Val::Px(current_x) = node.left {
           if (current_x - target_x).abs() > 0.1 { node.left = Val::Px(target_x); }
       } else { node.left = Val::Px(target_x); }

       if let Val::Px(current_y) = node.top {
           if (current_y - target_y).abs() > 0.1 { node.top = Val::Px(target_y); }
       } else { node.top = Val::Px(target_y); }
   }
}

/// Runs every frame during Drag: provides tint (Green/Red) and rotation
fn update_drag_visuals(
   mut interaction: ResMut<InteractionState>,
   mut q_dragged: Query<(&mut Node, &mut BackgroundColor, &mut ItemRotation, &InventoryItem, Option<&Bag>)>,
   grid_state: Res<InventoryGridState>,
   input: Res<ButtonInput<KeyCode>>,
) {
   let Some(entity) = interaction.dragged_entity else { return; };

   if let Ok((mut node, mut bg, mut rot, item_def, is_bag)) = q_dragged.get_mut(entity) {

       // 1. Handle rotation (R)
       if input.just_pressed(KeyCode::KeyR) {
           rot.0 = (rot.0 + 1) % 4;
           // Visually swap width/height for preview
           // Note: works for rectangles. Complex shapes need texture/mesh rotation.
           let temp = node.width;
           node.width = node.height;
           node.height = temp;
       }

       // 2. Calculate "Ghost" position
       let current_left = if let Val::Px(v) = node.left { v } else { 0.0 };
       let current_top = if let Val::Px(v) = node.top { v } else { 0.0 };

       let grid_x = (current_left / GRID_STEP).round() as i32;
       let grid_y = (current_top / GRID_STEP).round() as i32;
       let target_pos = IVec2::new(grid_x, grid_y);

       // 3. Real-time Validation
       let is_valid = if is_bag.is_some() {
           grid_state.can_place_bag(&item_def.base_shape, target_pos, rot.0, Some(entity))
       } else {
           grid_state.can_place_item(&item_def.base_shape, target_pos, rot.0, Some(entity))
       };

       // 4. Apply Tint
       if is_valid {
           *bg = BackgroundColor(Color::srgba(0.5, 1.0, 0.5, 0.8)); // Translucent Green
       } else {
           *bg = BackgroundColor(Color::srgba(1.0, 0.5, 0.5, 0.8)); // Translucent Red
       }
   }
}

// ============================================================================
// INITIALIZATION AND UTILITIES
// ============================================================================

fn setup_inventory_ui(mut commands: Commands) {
   // Root screen
   commands.spawn((
       Node {
           width: Val::Percent(100.0),
           height: Val::Percent(100.0),
           display: Display::Flex,
           flex_direction: FlexDirection::Column,
           align_items: AlignItems::Center,
           justify_content: JustifyContent::Center,
          ..default()
       },
       InventoryUiRoot,
       BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
   )).with_children(|parent| {

       parent.spawn((
           Text::new("Inventory Mode (Drag to Move, R to Rotate)"),
           TextFont { font_size: 20.0,..default() },
           TextColor(Color::WHITE),
           Node { margin: UiRect::bottom(Val::Px(20.0)),..default() }
       ));

       // Grid Container (Reference Frame)
       parent.spawn((
           Node {
               width: Val::Px(800.0),
               height: Val::Px(600.0),
               position_type: PositionType::Relative, // Important: children positioned relative to this
               border: UiRect::all(Val::Px(4.0)),
              ..default()
           },
           BorderColor(Color::WHITE),
           BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
           InventoryGridContainer,
       ));
   });
}

fn cleanup_inventory(
   mut commands: Commands,
   q: Query<Entity, With<InventoryUiRoot>>,
) {
   for e in q.iter() { commands.entity(e).despawn_recursive(); }
}

/// Helper for spawning items. Used by other plugins (Shop, LoadGame).
pub fn spawn_item_entity(
   commands: &mut Commands,
   parent: Entity,
   def: &ItemDefinition,
   pos: IVec2,
   rot: u8,
   _grid_state: &mut InventoryGridState,
) {
   // Determine pixels size with rotation
   let (w, h) = if rot % 2 == 0 { (def.width, def.height) } else { (def.height, def.width) };

   let width_px = w as f32 * GRID_STEP - CELL_GAP;
   let height_px = h as f32 * GRID_STEP - CELL_GAP;
   let x_px = pos.x as f32 * GRID_STEP;
   let y_px = pos.y as f32 * GRID_STEP;

   let is_bag = matches!(def.item_type, ItemType::Bag {..});
   // Bags lower (Z=1), items higher (Z=10)
   let color = if is_bag { Color::srgb(0.6, 0.4, 0.2) } else { Color::srgb(0.3, 0.3, 0.8) };
   let z = if is_bag { 1 } else { 10 };

   let id = commands.spawn((
       Node {
           position_type: PositionType::Absolute,
           left: Val::Px(x_px),
           top: Val::Px(y_px),
           width: Val::Px(width_px),
           height: Val::Px(height_px),
           border: UiRect::all(Val::Px(1.0)),
           // Important: padding and margins can mess up calculations, use absolute positioning
          ..default()
       },
       BackgroundColor(color),
       BorderColor(Color::BLACK),
       InventoryItem {
           item_id: def.id.clone(),
           base_shape: def.shape.clone(),
           width: def.width,
           height: def.height,
       },
       GridPosition(pos),
       ItemRotation(rot),
       ZIndex(z),
       PickingBehavior::default(), // Enable Picking explicitly
   )).with_children(|p| {
       p.spawn((
           Text::new(&def.name),
           TextFont { font_size: 10.0,..default() },
           TextColor(Color::WHITE),
           PickingBehavior::IGNORE, // Text should not capture clicks
       ));
   }).id();

   if is_bag {
       commands.entity(id).insert(Bag { provided_slots: def.shape.clone() });
   }

   commands.entity(parent).add_child(id);
}
