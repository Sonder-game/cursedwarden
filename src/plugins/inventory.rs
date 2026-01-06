use bevy::prelude::*;
use bevy::utils::HashMap;
use crate::plugins::core::GameState;
use crate::plugins::items::{ItemDefinition, ItemType, SynergyEffect, StatType};
use crate::plugins::metagame::PersistentInventory;

/// Plugin managing all inventory logic, grid, and interaction.
pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
   fn build(&self, app: &mut App) {
       app
           // Resources: Store grid state and drag data
          .init_resource::<InventoryGridState>()
          .init_resource::<DragState>()
           // Events: Signal changes for stat recalc
          .add_event::<InventoryChangedEvent>()
          .add_event::<ItemSpawnedEvent>() // Compat
           // Lifecycle systems
          .add_systems(OnEnter(GameState::EveningPhase), setup_inventory_ui)
          .add_systems(OnExit(GameState::EveningPhase), cleanup_inventory)
           // Update systems (only in EveningPhase)
          .add_systems(
               Update,
               (
                   update_grid_visuals,       // ECS -> UI Sync
                   handle_keyboard_rotation,  // R key rotation
                   debug_grid_gizmos,         // Visual debug (optional)
               ).run_if(in_state(GameState::EveningPhase))
           )
           // Bevy Picking Observers: New Event System for Drag & Drop (Bevy 0.15)
          .add_observer(on_drag_start)
          .add_observer(on_drag)
          .add_observer(on_drag_end);
   }
}

// ============================================================================
// COMPONENTS
// ============================================================================

/// Main item component. Stores ID and shape.
#[derive(Component)]
pub struct InventoryItem {
   pub item_id: String,
   /// List of relative coordinates occupied by item.
   /// (0,0) is top-left (Anchor).
   pub shape: Vec<IVec2>,
}

/// Bag component. Bag is an item that CREATES slots.
#[derive(Component)]
pub struct Bag {
   /// Shape of provided slots (relative to Anchor).
   pub provided_slots: Vec<IVec2>,
}

/// Logical grid position. Single source of truth for logic.
/// IVec2(x, y). Y grows downwards.
#[derive(Component, Clone, Copy, Debug)]
pub struct GridPosition(pub IVec2);

/// Current rotation: 0=0°, 1=90°, 2=180°, 3=270°.
#[derive(Component, Clone, Copy, Debug)]
pub struct ItemRotation(pub u8);

/// Marker indicating item is in "Storage" (Limbo) zone.
#[derive(Component)]
pub struct InStorage;

/// Markers for UI nodes
#[derive(Component)]
struct InventoryRoot;
#[derive(Component)]
pub struct InventoryGridContainer; // Active inventory zone
#[derive(Component)]
pub struct StorageContainer;       // "Limbo" zone

// Compat Components
#[derive(Component)]
pub struct Item;

#[derive(Component, Debug)]
pub struct ItemSize {
    pub width: i32,
    pub height: i32,
}

#[derive(Event)]
pub struct ItemSpawnedEvent(pub Entity);

// ============================================================================
// RESOURCES
// ============================================================================

/// Global grid state. Used for fast collision checks (O(1)).
#[derive(Resource, Default)]
pub struct InventoryGridState {
   /// Slot map. Key - coordinate. Value - slot data.
   pub slots: HashMap<IVec2, SlotData>,
   /// Bounds of active zone (to limit bag movement).
   pub bounds: IRect,
}

#[derive(Clone, Copy, Debug)]
pub struct SlotData {
   /// Entity ID of the bag that created this slot.
   pub bag_entity: Entity,
   /// Entity ID of the item occupying this slot (or None).
   pub occupier: Option<Entity>,
}

/// Current drag state.
#[derive(Resource, Default)]
pub struct DragState {
   /// Original position (for rollback on invalid drop).
   pub original_pos: Option<IVec2>,
   pub original_rotation: Option<u8>,
   pub was_in_storage: bool,
   /// If dragging a bag, this stores IDs of items inside it.
   pub attached_items: Vec<Entity>,
}

#[derive(Event)]
pub struct InventoryChangedEvent;

// ============================================================================
// CONSTANTS (Visual Style)
// ============================================================================
const SLOT_SIZE: f32 = 64.0;
const SLOT_GAP: f32 = 2.0;
const TOTAL_CELL_SIZE: f32 = SLOT_SIZE + SLOT_GAP;
// Y offset for storage zone (visually below main grid)
const STORAGE_OFFSET_Y: i32 = 10;

// ============================================================================
// GRID LOGIC (ALGORITHMS)
// ============================================================================

impl InventoryGridState {
    // Helper to rotate a shape (Static version for external use)
    pub fn get_rotated_shape(shape: &Vec<IVec2>, rotation_step: u8) -> Vec<IVec2> {
        rotate_shape(shape, rotation_step)
    }

   /// Full rebuild of slot map. Called after any change.
   /// Guarantees data integrity and solves desync issues.
   pub fn rebuild(
       &mut self,
       q_bags: &Query<(Entity, &GridPosition, &ItemRotation, &Bag), Without<InStorage>>,
       q_items: &Query<(Entity, &GridPosition, &ItemRotation, &InventoryItem), (Without<Bag>, Without<InStorage>)>,
   ) {
       self.slots.clear();
       self.bounds = IRect::new(0, 0, 0, 0);

       // 1. Project all bags onto grid (create valid slots)
       for (bag_entity, bag_pos, bag_rot, bag) in q_bags.iter() {
           let shape = rotate_shape(&bag.provided_slots, bag_rot.0);
           for offset in shape {
               let slot_pos = bag_pos.0 + offset;
               // If bags overlap, last one "wins" (could add overlap prevention logic)
               self.slots.insert(slot_pos, SlotData {
                   bag_entity,
                   occupier: None,
               });
               // Expand grid bounds
               self.bounds.max = self.bounds.max.max(slot_pos);
               self.bounds.min = self.bounds.min.min(slot_pos);
           }
       }

       // 2. Place items into slots
       for (item_entity, item_pos, item_rot, item) in q_items.iter() {
           let shape = rotate_shape(&item.shape, item_rot.0);
           for offset in shape {
               let cell_pos = item_pos.0 + offset;

               if let Some(slot) = self.slots.get_mut(&cell_pos) {
                   if slot.occupier.is_some() {
                       warn!("Collision! Cell {:?} already occupied.", cell_pos);
                   }
                   slot.occupier = Some(item_entity);
               } else {
                   // Item is floating (outside bag). Valid only during drag.
                   // If persisted, it's a logic error.
               }
           }
       }
   }

   /// Checks if ITEM can be placed at coordinates.
   pub fn can_place_item(
       &self,
       shape: &Vec<IVec2>,
       pos: IVec2,
       rot: u8,
       exclude_entity: Option<Entity>,
       target_is_storage: bool,
   ) -> bool {
       // Storage is always valid placement (simplified: infinite capacity / no bag requirement)
       // TODO: Implement proper storage grid/collision
       if target_is_storage {
           return true;
       }

       let rotated_shape = rotate_shape(shape, rot);

       for offset in rotated_shape {
           let target_pos = pos + offset;

           match self.slots.get(&target_pos) {
               Some(slot) => {
                   // Slot exists (on a bag). Check occupancy.
                   if let Some(occupier) = slot.occupier {
                       // If occupied by someone else - collision.
                       if Some(occupier) != exclude_entity {
                           return false;
                       }
                   }
               },
               None => return false, // No bag under item -> Cannot place.
           }
       }
       true
   }

   /// Checks if BAG can be placed. Bags must not overlap other bags.
   pub fn can_place_bag(
       &self,
       bag_shape: &Vec<IVec2>,
       pos: IVec2,
       rot: u8,
       exclude_bag: Option<Entity>,
   ) -> bool {
       let rotated_shape = rotate_shape(bag_shape, rot);
       for offset in rotated_shape {
           let target_pos = pos + offset;
           if let Some(slot) = self.slots.get(&target_pos) {
               if Some(slot.bag_entity) != exclude_bag {
                   return false; // Overlaps another bag
               }
           }
       }
       true
   }

   // Compatibility helper
   pub fn find_free_spot(&self, def: &ItemDefinition) -> Option<IVec2> {
        // Iterate through all known slots bounds
        let min = self.bounds.min;
        let max = self.bounds.max;

        // Naive search
        for y in min.y..=max.y {
            for x in min.x..=max.x {
                let pos = IVec2::new(x, y);
                // Assume 0 rotation for auto-placement
                if self.can_place_item(&def.shape, pos, 0, None, false) {
                    return Some(pos);
                }
            }
        }
        None
   }
}

/// Discrete rotation math (90 deg CW)
fn rotate_shape(shape: &Vec<IVec2>, rot: u8) -> Vec<IVec2> {
   let steps = rot % 4;
   if steps == 0 { return shape.clone(); }

   shape.iter().map(|p| {
       let mut v = *p;
       for _ in 0..steps {
           // Rotation formula: (x, y) -> (-y, x)
           v = IVec2::new(-v.y, v.x);
       }
       v
   }).collect()
}

// ============================================================================
// INTERACTION SYSTEM (BEVY PICKING OBSERVERS)
// ============================================================================

/// Drag Start (LMB Down)
fn on_drag_start(
   trigger: Trigger<Pointer<DragStart>>,
   mut commands: Commands,
   q_items: Query<(Entity, &GridPosition, &ItemRotation, Option<&Bag>, Has<InStorage>), With<InventoryItem>>,
   mut drag_state: ResMut<DragState>,
   mut q_node: Query<(&mut ZIndex, &Node)>,
   grid_state: Res<InventoryGridState>,
) {
   let entity = trigger.entity();

   if let Ok((_e, grid_pos, rot, is_bag, in_storage)) = q_items.get(entity) {
       // 1. Save state for rollback (Undo)
       drag_state.original_pos = Some(grid_pos.0);
       drag_state.original_rotation = Some(rot.0);
       drag_state.was_in_storage = in_storage;
       drag_state.attached_items.clear();

       // 2. BAG DRAG LOGIC
       if is_bag.is_some() && !in_storage {
           // Find all items sitting in slots provided by this bag
           for (_slot_pos, slot_data) in &grid_state.slots {
               if slot_data.bag_entity == entity {
                   if let Some(occupier) = slot_data.occupier {
                       if !drag_state.attached_items.contains(&occupier) {
                           drag_state.attached_items.push(occupier);
                       }
                   }
               }
           }
       }

       // 3. Visual Feedback: Raise Z-Index
       if let Ok((mut z_index, _)) = q_node.get_mut(entity) {
           *z_index = ZIndex(100);
       }

       // 4. CRITICAL: Ignore Picking for item itself during drag
       // Allows mouse ray to pierce through item and hit the grid/slots underneath
       commands.entity(entity).insert(PickingBehavior::IGNORE);
   }
}

/// Drag Process (Mouse Move)
fn on_drag(
   trigger: Trigger<Pointer<Drag>>,
   mut q_node: Query<&mut Node>,
) {
   let entity = trigger.entity();
   let drag_event = trigger.event();

   if let Ok(mut node) = q_node.get_mut(entity) {
       // Update visual coordinates (Style). Logic coordinates (GridPosition) untouched.
       if let Val::Px(left) = node.left {
           node.left = Val::Px(left + drag_event.delta.x);
       }
       if let Val::Px(top) = node.top {
           node.top = Val::Px(top + drag_event.delta.y);
       }
   }
}

/// Drag End (LMB Up)
fn on_drag_end(
   trigger: Trigger<Pointer<DragEnd>>,
   mut commands: Commands,
   // ParamSet to resolve borrow conflicts
   mut queries: ParamSet<(
       Query<(Entity, &mut GridPosition, &mut ItemRotation, &InventoryItem, &Node, Option<&Bag>, Has<InStorage>)>, // Mutable
       (
           Query<(Entity, &GridPosition, &ItemRotation, &Bag), Without<InStorage>>, // Bags Read-Only
           Query<(Entity, &GridPosition, &ItemRotation, &InventoryItem), (Without<Bag>, Without<InStorage>)> // Items Read-Only
       )
   )>,
   mut grid_state: ResMut<InventoryGridState>,
   drag_state: Res<DragState>,
   mut ev_changed: EventWriter<InventoryChangedEvent>,
) {
   let entity = trigger.entity();

   // 1. Restore interactivity
   commands.entity(entity).remove::<PickingBehavior>();

   let mut success = false;

   // Scope for mutable access
   {
       let mut q_mutable = queries.p0();
       if let Ok((_, mut grid_pos, mut rot, item_def, node, is_bag, _)) = q_mutable.get_mut(entity) {

           // 2. Snapping calculation
           // Convert UI Node coords to grid indices
           let current_left = if let Val::Px(v) = node.left { v } else { 0.0 };
           let current_top = if let Val::Px(v) = node.top { v } else { 0.0 };

           // Determine if dropping into Storage zone
           // (Simple Y threshold check for now)
           let is_storage_drop = current_top > 400.0; // Adjusted threshold based on setup_inventory_ui

           let target_x = (current_left / TOTAL_CELL_SIZE).round() as i32;
           let target_y = (current_top / TOTAL_CELL_SIZE).round() as i32;
           let target_pos = IVec2::new(target_x, target_y);

           // 3. Validation
           let mut valid = false;

           if is_storage_drop {
               // Storage Drop
               commands.entity(entity).insert(InStorage);
               valid = true;
           } else {
               // Grid Drop
               commands.entity(entity).remove::<InStorage>();

               if let Some(bag) = is_bag {
                   // Moving Bag
                   if grid_state.can_place_bag(&bag.provided_slots, target_pos, rot.0, Some(entity)) {
                       valid = true;
                   }
               } else {
                   // Moving Item
                   if grid_state.can_place_item(&item_def.shape, target_pos, rot.0, Some(entity), false) {
                       valid = true;
                   }
               }
           }

           // 4. Commit or Rollback
           if valid {
               // SUCCESS
               grid_pos.0 = target_pos;
               ev_changed.send(InventoryChangedEvent);
               success = true;
           } else {
               // ROLLBACK
               if let Some(orig) = drag_state.original_pos {
                   grid_pos.0 = orig;
               }
               if let Some(orig_rot) = drag_state.original_rotation {
                   rot.0 = orig_rot;
               }
               // Restore Storage state
               if drag_state.was_in_storage {
                    commands.entity(entity).insert(InStorage);
               } else {
                    commands.entity(entity).remove::<InStorage>();
               }
           }
       }
   }

   // Handle Bag Passengers
   if success {
       let mut q_mutable = queries.p0(); // Re-borrow
       // Calculate delta
       if let Ok((_, grid_pos, _, _, _, is_bag, _)) = q_mutable.get(entity) {
           if is_bag.is_some() {
               let delta = grid_pos.0 - drag_state.original_pos.unwrap_or(grid_pos.0);
               if delta != IVec2::ZERO {
                   for attached_entity in &drag_state.attached_items {
                        if let Ok((_, mut item_pos, _, _, _, _, _)) = q_mutable.get_mut(*attached_entity) {
                            item_pos.0 += delta;
                        }
                   }
               }
           }
       }
   }

   // 5. Rebuild grid state for next frame
   let (q_bags, q_items) = queries.p1();
   grid_state.rebuild(&q_bags, &q_items);
}

// ============================================================================
// VISUAL SYNC
// ============================================================================

/// Syncs UI Node position with logical GridPosition.
/// Runs every frame to ensure smoothness and snap correction.
fn update_grid_visuals(
   mut q_items: Query<(Entity, &GridPosition, &mut Node, &mut ZIndex, Option<&PickingBehavior>), (With<InventoryItem>, Changed<GridPosition>)>,
) {
   for (_entity, pos, mut node, mut z_index, picking) in q_items.iter_mut() {
       // Don't touch if currently dragging
       if let Some(behavior) = picking {
           if *behavior == PickingBehavior::IGNORE {
               continue;
           }
       }

       // Snap to grid
       node.left = Val::Px(pos.0.x as f32 * TOTAL_CELL_SIZE);
       node.top = Val::Px(pos.0.y as f32 * TOTAL_CELL_SIZE);

       // Reset Z-Index
       *z_index = ZIndex(10);
   }
}

/// Rotation handling (R key)
fn handle_keyboard_rotation(
   input: Res<ButtonInput<KeyCode>>,
   mut q_items: Query<(&mut ItemRotation, &mut Node), With<PickingBehavior>>, // Only dragging items (IGNORE)
) {
   if input.just_pressed(KeyCode::KeyR) {
       for (mut rot, mut node) in q_items.iter_mut() {
           rot.0 = (rot.0 + 1) % 4;
           // Visual rotation: swap width/height
           let temp = node.width;
           node.width = node.height;
           node.height = temp;
       }
   }
}

// ============================================================================
// UI INITIALIZATION
// ============================================================================

fn setup_inventory_ui(mut commands: Commands) {
   // Root
   commands.spawn((
       Node {
           width: Val::Percent(100.0),
           height: Val::Percent(100.0),
           justify_content: JustifyContent::FlexStart, // Top-down
           align_items: AlignItems::Center,
           flex_direction: FlexDirection::Column,
           ..default()
       },
       InventoryRoot,
   )).with_children(|parent| {
       // 1. Active Grid Zone
       parent.spawn((
           Node {
               width: Val::Px(800.0),
               height: Val::Px(400.0),
               position_type: PositionType::Relative,
               border: UiRect::all(Val::Px(2.0)),
               margin: UiRect::bottom(Val::Px(20.0)),
               ..default()
           },
           InventoryGridContainer,
           BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
       ));

       // 2. Storage Zone (Limbo)
       parent.spawn((
            Node {
               width: Val::Px(800.0),
               height: Val::Px(200.0),
               position_type: PositionType::Relative,
               border: UiRect::all(Val::Px(2.0)),
               ..default()
           },
           StorageContainer,
           BackgroundColor(Color::srgb(0.15, 0.15, 0.25)), // Blueish
       )).with_children(|p| {
            p.spawn((
               Text::new("STORAGE (LIMBO)"),
               TextFont { font_size: 20.0, ..default() },
               TextColor(Color::WHITE),
               Node { position_type: PositionType::Absolute, top: Val::Px(5.0), left: Val::Px(5.0), ..default() },
            ));
       });
   });
}

fn cleanup_inventory(mut commands: Commands, q: Query<Entity, With<InventoryRoot>>) {
   for e in q.iter() {
       commands.entity(e).despawn_recursive();
   }
}

fn debug_grid_gizmos(_gizmos: Gizmos) {}

// ============================================================================
// HELPERS (Compat)
// ============================================================================

pub fn spawn_item_entity(
    commands: &mut Commands,
    container: Entity,
    def: &ItemDefinition,
    pos: IVec2,
    rotation: u8,
    grid_state: &mut InventoryGridState, // We update this to ensure sequential spawns work
) {
    // Calculate pixel size
    let (width_slots, height_slots) = if rotation % 2 == 0 {
        (def.width, def.height)
    } else {
        (def.height, def.width)
    };

    let w = width_slots as f32 * TOTAL_CELL_SIZE - SLOT_GAP;
    let h = height_slots as f32 * TOTAL_CELL_SIZE - SLOT_GAP;

    let is_bag = matches!(def.item_type, ItemType::Bag { .. });
    let z_idx = if is_bag { ZIndex(1) } else { ZIndex(10) };
    let bg_color = if is_bag { Color::srgb(0.4, 0.2, 0.1) } else { Color::srgb(0.5, 0.5, 0.8) };

    // Check if bag and get provided slots if so
    let bag_comp = if is_bag {
        Some(Bag { provided_slots: def.shape.clone() })
    } else {
        None
    };

    let item_shape = def.shape.clone();

    commands.entity(container).with_children(|p| {
        let mut builder = p.spawn((
            Node {
                width: Val::Px(w),
                height: Val::Px(h),
                position_type: PositionType::Absolute,
                left: Val::Px(pos.x as f32 * TOTAL_CELL_SIZE),
                top: Val::Px(pos.y as f32 * TOTAL_CELL_SIZE),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(bg_color),
            InventoryItem {
                item_id: def.id.clone(),
                shape: item_shape.clone()
            },
            GridPosition(pos),
            ItemRotation(rotation),
            z_idx,
            PickingBehavior::default(),
            // Compat markers
            Item,
            ItemSize { width: width_slots as i32, height: height_slots as i32 },
        ));

        if let Some(bag) = bag_comp {
            builder.insert(bag);
        }

        builder.with_children(|inner| {
            inner.spawn((
                Text::new(&def.name),
                TextFont { font_size: 12.0, ..default() },
                TextColor(Color::WHITE),
            ));
        });

        let entity = builder.id();

        // Update grid state immediately for sequential spawns (e.g. Shop buying)
        // Note: This is a partial update. Full rebuild happens next frame.
        // We do this to support find_free_spot calls in the same frame.
        if is_bag {
             let shape = rotate_shape(&item_shape, rotation);
             for offset in shape {
                 let slot_pos = pos + offset;
                 grid_state.slots.insert(slot_pos, SlotData { bag_entity: entity, occupier: None });
                 grid_state.bounds.max = grid_state.bounds.max.max(slot_pos);
                 grid_state.bounds.min = grid_state.bounds.min.min(slot_pos);
             }
        } else {
             let shape = rotate_shape(&item_shape, rotation);
             for offset in shape {
                 let cell_pos = pos + offset;
                 if let Some(slot) = grid_state.slots.get_mut(&cell_pos) {
                     slot.occupier = Some(entity);
                 }
             }
        }
    });
}

pub struct CombatStats {
    pub attack: f32,
    pub defense: f32,
    pub speed: f32,
    pub health: f32,
}

pub fn calculate_combat_stats(
    inventory: &PersistentInventory,
    item_db: &crate::plugins::items::ItemDatabase,
) -> CombatStats {
    let mut stats = CombatStats {
        attack: 0.0,
        defense: 0.0,
        speed: 0.0,
        health: 0.0,
    };

    // Reconstruct Grid State locally
    let mut temp_grid = InventoryGridState::default();

    // 1. Place Bags
    let mut bag_map = HashMap::new();
    for (i, saved_item) in inventory.items.iter().enumerate() {
        if let Some(def) = item_db.items.get(&saved_item.item_id) {
            if matches!(def.item_type, ItemType::Bag { .. }) {
                let entity = Entity::from_raw(i as u32);
                let pos = IVec2::new(saved_item.grid_x, saved_item.grid_y);
                let shape = InventoryGridState::get_rotated_shape(&def.shape, saved_item.rotation);

                bag_map.insert(entity, def.clone());

                for offset in shape {
                    let slot_pos = pos + offset;
                    temp_grid.slots.insert(slot_pos, SlotData {
                        bag_entity: entity,
                        occupier: None,
                    });
                }
            }
        }
    }

    // 2. Place Items
    let mut item_entities = Vec::new();
    for (i, saved_item) in inventory.items.iter().enumerate() {
        if let Some(def) = item_db.items.get(&saved_item.item_id) {
            if !matches!(def.item_type, ItemType::Bag { .. }) {
                let entity = Entity::from_raw(i as u32);
                let pos = IVec2::new(saved_item.grid_x, saved_item.grid_y);
                let rot = saved_item.rotation;
                
                item_entities.push((entity, def, pos, rot));

                let shape = InventoryGridState::get_rotated_shape(&def.shape, rot);
                for offset in shape {
                    let slot_pos = pos + offset;
                    if let Some(slot) = temp_grid.slots.get_mut(&slot_pos) {
                        slot.occupier = Some(entity);
                    }
                }
            }
        }
    }

    // 3. Calculate Stats & Synergies
    let item_lookup: HashMap<Entity, &ItemDefinition> = item_entities.iter().map(|(e, d, _, _)| (*e, *d)).collect();
    let mut bonuses: HashMap<Entity, CombatStats> = HashMap::new();

    for (entity, def, pos, rot) in &item_entities {
        for synergy in &def.synergies {
            let shape = InventoryGridState::get_rotated_shape(&vec![synergy.offset], *rot);
            if shape.is_empty() { continue; }
            let target_pos = *pos + shape[0];

            if let Some(slot) = temp_grid.slots.get(&target_pos) {
                if let Some(target_entity) = slot.occupier {
                    if let Some(target_def) = item_lookup.get(&target_entity) {
                        if synergy.target_tags.iter().any(|req| target_def.tags.contains(req)) {
                            match synergy.effect {
                                SynergyEffect::BuffSelf { stat, value } => {
                                    let b = bonuses.entry(*entity).or_insert(CombatStats { attack: 0.0, defense: 0.0, speed: 0.0, health: 0.0 });
                                    match stat {
                                        StatType::Attack => b.attack += value,
                                        StatType::Defense => b.defense += value,
                                        StatType::Speed => b.speed += value,
                                        StatType::Health => b.health += value,
                                    }
                                },
                                SynergyEffect::BuffTarget { stat, value } => {
                                    let b = bonuses.entry(target_entity).or_insert(CombatStats { attack: 0.0, defense: 0.0, speed: 0.0, health: 0.0 });
                                    match stat {
                                        StatType::Attack => b.attack += value,
                                        StatType::Defense => b.defense += value,
                                        StatType::Speed => b.speed += value,
                                        StatType::Health => b.health += value,
                                    }
                                },
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }

    // Sum Final Stats
    for (entity, def, _, _) in &item_entities {
        stats.attack += def.attack;
        stats.defense += def.defense;
        stats.speed += def.speed;

        if let Some(bonus) = bonuses.get(entity) {
            stats.attack += bonus.attack;
            stats.defense += bonus.defense;
            stats.speed += bonus.speed;
            stats.health += bonus.health;
        }
    }

    stats
}
