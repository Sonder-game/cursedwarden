use bevy::prelude::*;
use bevy::utils::HashMap;
use crate::plugins::core::GameState;
use crate::plugins::items::{ItemDefinition, ItemType, SynergyEffect, StatType};
use crate::plugins::metagame::PersistentInventory;

pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
   fn build(&self, app: &mut App) {
       app
           // Resources
          .init_resource::<InventoryGridState>()
          .init_resource::<DragState>()
           // Events
          .add_event::<InventoryChangedEvent>()
          .add_event::<ItemSpawnedEvent>()
           // Systems
          .add_systems(OnEnter(GameState::EveningPhase), setup_inventory_ui)
          .add_systems(OnExit(GameState::EveningPhase), cleanup_inventory)
           // Update Systems
          .add_systems(
               Update,
               (
                   update_grid_visuals,
                   handle_keyboard_rotation,
                   debug_grid_gizmos,
               ).run_if(in_state(GameState::EveningPhase))
           )
           // Observers
          .add_observer(on_drag_start)
          .add_observer(on_drag)
          .add_observer(on_drag_end);
   }
}

// ============================================================================
// COMPONENTS
// ============================================================================

/// Marker for backward compatibility and querying
#[derive(Component)]
pub struct Item;

/// Marker for backward compatibility
#[derive(Component, Debug)]
pub struct ItemSize {
    pub width: i32,
    pub height: i32,
}

/// Event triggered when item is spawned
#[derive(Event)]
pub struct ItemSpawnedEvent(pub Entity);

/// Main component for inventory item
#[derive(Component)]
pub struct InventoryItem {
   pub item_id: String,
   /// Base shape (list of offsets from 0,0)
   pub shape: Vec<IVec2>,
}

#[derive(Component)]
pub struct Bag {
   pub provided_slots: Vec<IVec2>,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct GridPosition(pub IVec2);

// Make sure to access 'value' for backward compat or just alias
#[derive(Component, Clone, Copy, Debug)]
pub struct ItemRotation(pub u8);

#[derive(Component)]
struct InventoryRoot;

/// Marker for the grid container area
#[derive(Component)]
pub struct InventoryGridContainer;

// ============================================================================
// RESOURCES AND STATE
// ============================================================================

#[derive(Resource, Default)]
pub struct InventoryGridState {
   pub slots: HashMap<IVec2, SlotData>,
   pub bounds: IRect,
}

#[derive(Clone, Copy, Debug)]
pub struct SlotData {
   pub bag_entity: Entity,
   pub occupier: Option<Entity>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CellState {
    Free,
    Occupied(Entity),
}

#[derive(Resource, Default)]
pub struct DragState {
   pub original_pos: Option<IVec2>,
   pub original_rotation: Option<u8>,
   pub attached_items: Vec<Entity>,
   pub drag_offset: Vec2,
}

#[derive(Event)]
pub struct InventoryChangedEvent;

// ============================================================================
// CONSTANTS
// ============================================================================

const SLOT_SIZE: f32 = 64.0;
const SLOT_GAP: f32 = 2.0;
const TOTAL_CELL_SIZE: f32 = SLOT_SIZE + SLOT_GAP;

// ============================================================================
// GRID LOGIC
// ============================================================================

impl InventoryGridState {
    // Helper to rotate a shape (Static version for external use)
   pub fn get_rotated_shape(shape: &Vec<IVec2>, rotation_step: u8) -> Vec<IVec2> {
       rotate_shape(shape, rotation_step)
   }

   pub fn rebuild(
       &mut self,
       q_bags: &Query<(Entity, &GridPosition, &ItemRotation, &Bag)>,
       q_items: &Query<(Entity, &GridPosition, &ItemRotation, &InventoryItem), Without<Bag>>,
   ) {
       self.slots.clear();
       self.bounds = IRect::new(0, 0, 0, 0);

       for (bag_entity, bag_pos, bag_rot, bag) in q_bags.iter() {
           let shape = rotate_shape(&bag.provided_slots, bag_rot.0);
           for offset in shape {
               let slot_pos = bag_pos.0 + offset;
               self.slots.insert(slot_pos, SlotData {
                   bag_entity,
                   occupier: None,
               });
               self.bounds.max = self.bounds.max.max(slot_pos);
               self.bounds.min = self.bounds.min.min(slot_pos);
           }
       }

       for (item_entity, item_pos, item_rot, item) in q_items.iter() {
           let shape = rotate_shape(&item.shape, item_rot.0);
           for offset in shape {
               let cell_pos = item_pos.0 + offset;
               if let Some(slot) = self.slots.get_mut(&cell_pos) {
                   if slot.occupier.is_some() {
                       warn!("Double occupancy at {:?} by item {:?}", cell_pos, item_entity);
                   }
                   slot.occupier = Some(item_entity);
               } else {
                   warn!("Item {:?} at {:?} is floating (no bag)", item_entity, cell_pos);
               }
           }
       }
   }

   pub fn can_place_item(
       &self,
       shape: &Vec<IVec2>,
       pos: IVec2,
       rot: u8,
       exclude_entity: Option<Entity>,
   ) -> bool {
       let rotated_shape = rotate_shape(shape, rot);
       for offset in rotated_shape {
           let target_pos = pos + offset;
           match self.slots.get(&target_pos) {
               Some(slot) => {
                   if let Some(occupier) = slot.occupier {
                       if Some(occupier) != exclude_entity {
                           return false;
                       }
                   }
               },
               None => return false,
           }
       }
       true
   }

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
                   return false;
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

        for y in min.y..=max.y {
            for x in min.x..=max.x {
                let pos = IVec2::new(x, y);
                if self.can_place_item(&def.shape, pos, 0, None) {
                    return Some(pos);
                }
            }
        }
        None
   }
}

fn rotate_shape(shape: &Vec<IVec2>, rot: u8) -> Vec<IVec2> {
   let steps = rot % 4;
   if steps == 0 { return shape.clone(); }

   shape.iter().map(|p| {
       let mut v = *p;
       for _ in 0..steps {
           v = IVec2::new(-v.y, v.x);
       }
       v
   }).collect()
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

pub fn spawn_item_entity(
    commands: &mut Commands,
    container: Entity,
    def: &ItemDefinition,
    pos: IVec2,
    rotation: u8,
    _grid_state: &mut InventoryGridState, // Note: Not used directly for validation here to avoid ownership issues, but kept signature
) {
    let (_min_x, _min_y, width_slots, height_slots) = calculate_bounding_box(&def.shape, rotation);

    let width_px = width_slots as f32 * TOTAL_CELL_SIZE - SLOT_GAP;
    let height_px = height_slots as f32 * TOTAL_CELL_SIZE - SLOT_GAP;

    // Position logic
    let left = pos.x as f32 * TOTAL_CELL_SIZE;
    let top = pos.y as f32 * TOTAL_CELL_SIZE;

    let is_bag = matches!(def.item_type, ItemType::Bag { .. });
    let z_idx = if is_bag { ZIndex(1) } else { ZIndex(10) };
    let bg_color = if is_bag { Color::srgb(0.4, 0.2, 0.1) } else { Color::srgb(0.5, 0.5, 0.8) };

    // Prepare shape/components
    let shape = def.shape.clone();

    let mut entity_cmds = commands.spawn((
        Node {
            width: Val::Px(width_px),
            height: Val::Px(height_px),
            position_type: PositionType::Absolute,
            left: Val::Px(left),
            top: Val::Px(top),
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        BackgroundColor(bg_color),
        BorderColor(Color::WHITE), // Default border
        InventoryItem {
            item_id: def.id.clone(),
            shape: shape.clone(),
        },
        GridPosition(pos),
        ItemRotation(rotation),
        z_idx,
        PickingBehavior::default(),
        // Add backward compat components
        Item,
        ItemSize { width: width_slots, height: height_slots },
    ));

    if is_bag {
        // Assume bags provide their shape as slots for now
        entity_cmds.insert(Bag { provided_slots: shape });
    }

    // Child text
    entity_cmds.with_children(|parent| {
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
    });

    let entity = entity_cmds.id();
    commands.entity(container).add_child(entity);
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

    // Reconstruct Grid State
    let mut temp_grid = InventoryGridState::default();

    // 1. Place Bags
    let mut bag_map = HashMap::new(); // Entity -> BagDef
    for (i, saved_item) in inventory.items.iter().enumerate() {
        if let Some(def) = item_db.items.get(&saved_item.item_id) {
            if matches!(def.item_type, ItemType::Bag { .. }) {
                // Fake Entity
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
    let mut item_entities = Vec::new(); // (Entity, Def, Pos, Rot)
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

    // This loop was unused in previous code and caused warnings.
    // Commented out as it seems redundant given the next loop does the full bonus calculation.
    /*
    for (entity, def, pos, rot) in &item_entities {
        let mut item_attack = def.attack;
        let mut item_defense = def.defense;
        let mut item_speed = def.speed;
        let mut item_health = 0.0; // Items usually don't have health but maybe?

        // Check Synergies
        for synergy in &def.synergies {
            let shape = InventoryGridState::get_rotated_shape(&vec![synergy.offset], *rot);
            if shape.is_empty() { continue; }
            let target_pos = *pos + shape[0];

            if let Some(slot) = temp_grid.slots.get(&target_pos) {
                if let Some(target_entity) = slot.occupier {
                    if let Some(target_def) = item_lookup.get(&target_entity) {
                        if synergy.target_tags.iter().any(|req| target_def.tags.contains(req)) {
                            // Apply Effect
                            match synergy.effect {
                                SynergyEffect::BuffSelf { stat, value } => {
                                    match stat {
                                        StatType::Attack => item_attack += value,
                                        StatType::Defense => item_defense += value,
                                        StatType::Speed => item_speed += value,
                                        StatType::Health => item_health += value,
                                    }
                                },
                                // Note: BuffTarget requires us to find the TARGET item in our iteration list and update it.
                                // Simplification: We only process Self buffs here OR we process bonuses in a separate pass?
                                // "BuffTarget" means THIS item buffs the NEIGHBOR.
                                // To do this correctly, we need a map of bonuses.
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }
    */

    // COMPLETE CALCULATION WITH MAP
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
        // stats.health += def.health; // if defined

        if let Some(bonus) = bonuses.get(entity) {
            stats.attack += bonus.attack;
            stats.defense += bonus.defense;
            stats.speed += bonus.speed;
            stats.health += bonus.health;
        }
    }

    stats
}

fn calculate_bounding_box(shape: &Vec<IVec2>, rotation_step: u8) -> (i32, i32, i32, i32) {
    let rotated_shape = rotate_shape(shape, rotation_step);
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


// ============================================================================
// SYSTEMS BEVY PICKING (DRAG & DROP)
// ============================================================================

fn on_drag_start(
   trigger: Trigger<Pointer<DragStart>>,
   mut commands: Commands,
   q_items: Query<(Entity, &GridPosition, &ItemRotation, Option<&Bag>), With<InventoryItem>>,
   mut drag_state: ResMut<DragState>,
   mut q_node: Query<(&mut ZIndex, &Node)>,
) {
   let entity = trigger.entity();

   if let Ok((_e, grid_pos, rot, is_bag)) = q_items.get(entity) {
       drag_state.original_pos = Some(grid_pos.0);
       drag_state.original_rotation = Some(rot.0);
       drag_state.attached_items.clear();

       if is_bag.is_some() {
           // Bag logic placeholder
       }

       if let Ok((mut z_index, _node)) = q_node.get_mut(entity) {
           *z_index = ZIndex(100);
       }

       commands.entity(entity).insert(PickingBehavior::IGNORE);
   }
}

fn on_drag(
   trigger: Trigger<Pointer<Drag>>,
   mut q_node: Query<&mut Node>,
) {
   let entity = trigger.entity();
   let drag_event = trigger.event();

   if let Ok(mut node) = q_node.get_mut(entity) {
       if let Val::Px(left) = node.left {
           node.left = Val::Px(left + drag_event.delta.x);
       }
       if let Val::Px(top) = node.top {
           node.top = Val::Px(top + drag_event.delta.y);
       }
   }
}

fn on_drag_end(
   trigger: Trigger<Pointer<DragEnd>>,
   mut commands: Commands,
   mut queries: ParamSet<(
        Query<(Entity, &mut GridPosition, &mut ItemRotation, &InventoryItem, &Node, Option<&Bag>)>,
        (
            Query<(Entity, &GridPosition, &ItemRotation, &Bag)>,
            Query<(Entity, &GridPosition, &ItemRotation, &InventoryItem), Without<Bag>>
        )
   )>,
   mut grid_state: ResMut<InventoryGridState>,
   drag_state: Res<DragState>,
   mut ev_changed: EventWriter<InventoryChangedEvent>,
) {
   let entity = trigger.entity();

   // Because we can't easily query mutably and immutably at same time for Bag Content logic,
   // we will do a simpler check.

   // Access mutable query for the dragged item
   {
       let mut q_items = queries.p0();
       if let Ok((_e, mut grid_pos, mut rot, item_def, node, is_bag)) = q_items.get_mut(entity) {
           commands.entity(entity).remove::<PickingBehavior>();
           let current_left = if let Val::Px(v) = node.left { v } else { 0.0 };
           let current_top = if let Val::Px(v) = node.top { v } else { 0.0 };

           let target_x = (current_left / TOTAL_CELL_SIZE).round() as i32;
           let target_y = (current_top / TOTAL_CELL_SIZE).round() as i32;
           let target_pos = IVec2::new(target_x, target_y);

           let mut valid = false;

           if let Some(bag) = is_bag {
               if grid_state.can_place_bag(&bag_def_to_shape(&bag.provided_slots), target_pos, rot.0, Some(entity)) {
                   valid = true;
               }
           } else {
               if grid_state.can_place_item(&item_def.shape, target_pos, rot.0, Some(entity)) {
                   valid = true;
               }
           }

           if valid {
               if is_bag.is_some() {
                   let delta = target_pos - drag_state.original_pos.unwrap_or(target_pos);
                   if delta != IVec2::ZERO {
                       // move_bag_contents placeholder
                   }
               }
               grid_pos.0 = target_pos;
               ev_changed.send(InventoryChangedEvent);
           } else {
               if let Some(orig) = drag_state.original_pos {
                   grid_pos.0 = orig;
               }
               if let Some(orig_rot) = drag_state.original_rotation {
                   rot.0 = orig_rot;
               }
           }
       }
   }

   // Access read-only queries for rebuild
   let (q_bags_ro, q_items_ro) = queries.p1();
   grid_state.rebuild_unified(&q_items_ro, &q_bags_ro);
}

impl InventoryGridState {
    pub fn rebuild_unified(
        &mut self,
        q_items: &Query<(Entity, &GridPosition, &ItemRotation, &InventoryItem), Without<Bag>>,
        q_bags: &Query<(Entity, &GridPosition, &ItemRotation, &Bag)>,
    ) {
        self.rebuild(q_bags, q_items);
    }
}

fn bag_def_to_shape(vec: &Vec<IVec2>) -> Vec<IVec2> {
   vec.clone()
}

// ============================================================================
// UPDATE SYSTEMS
// ============================================================================

fn update_grid_visuals(
   mut q_items: Query<(Entity, &GridPosition, &mut Node, &mut ZIndex, Option<&PickingBehavior>), (With<InventoryItem>, Changed<GridPosition>)>,
) {
   for (_entity, pos, mut node, mut z_index, picking) in q_items.iter_mut() {
       if let Some(behavior) = picking {
           if *behavior == PickingBehavior::IGNORE {
               continue;
           }
       }
       node.left = Val::Px(pos.0.x as f32 * TOTAL_CELL_SIZE);
       node.top = Val::Px(pos.0.y as f32 * TOTAL_CELL_SIZE);
       *z_index = ZIndex(10);
   }
}

fn handle_keyboard_rotation(
   input: Res<ButtonInput<KeyCode>>,
   mut q_items: Query<(&mut ItemRotation, &mut Node, &InventoryItem, &PickingBehavior)>,
) {
   if input.just_pressed(KeyCode::KeyR) {
       for (mut rot, mut node, _item, _) in q_items.iter_mut() {
           rot.0 = (rot.0 + 1) % 4;
           let temp = node.width;
           node.width = node.height;
           node.height = temp;
       }
   }
}

fn setup_inventory_ui(mut commands: Commands) {
   commands.spawn((
       Node {
           width: Val::Percent(100.0),
           height: Val::Percent(100.0),
           justify_content: JustifyContent::Center,
           align_items: AlignItems::Center,
           ..default()
       },
       InventoryRoot,
   )).with_children(|parent| {
       parent.spawn((
           Node {
               width: Val::Px(800.0),
               height: Val::Px(600.0),
               position_type: PositionType::Relative,
               border: UiRect::all(Val::Px(2.0)),
               ..default()
           },
           InventoryGridContainer, // Add marker
       )).with_children(|grid_area| {
           spawn_test_bag(grid_area, IVec2::new(2, 2));
           spawn_test_item(grid_area, IVec2::new(2, 2));
       });
   });
}

fn spawn_test_bag(parent: &mut ChildBuilder, pos: IVec2) {
   let shape = vec![IVec2::new(0,0), IVec2::new(1,0), IVec2::new(0,1), IVec2::new(1,1)];
   parent.spawn((
       InventoryItem { item_id: "bag_starter".into(), shape: shape.clone() },
       Bag { provided_slots: shape },
       GridPosition(pos),
       ItemRotation(0),
       Node {
           position_type: PositionType::Absolute,
           width: Val::Px(2.0 * TOTAL_CELL_SIZE - SLOT_GAP),
           height: Val::Px(2.0 * TOTAL_CELL_SIZE - SLOT_GAP),
           left: Val::Px(pos.x as f32 * TOTAL_CELL_SIZE),
           top: Val::Px(pos.y as f32 * TOTAL_CELL_SIZE),
           ..default()
       },
       BackgroundColor(Color::srgb(0.5, 0.3, 0.1)),
       PickingBehavior::default(),
   ));
}

fn spawn_test_item(parent: &mut ChildBuilder, pos: IVec2) {
   let shape = vec![IVec2::new(0,0), IVec2::new(0,1)];
   parent.spawn((
       InventoryItem { item_id: "sword".into(), shape },
       GridPosition(pos),
       ItemRotation(0),
       Node {
           position_type: PositionType::Absolute,
           width: Val::Px(1.0 * TOTAL_CELL_SIZE - SLOT_GAP),
           height: Val::Px(2.0 * TOTAL_CELL_SIZE - SLOT_GAP),
           left: Val::Px(pos.x as f32 * TOTAL_CELL_SIZE),
           top: Val::Px(pos.y as f32 * TOTAL_CELL_SIZE),
           ..default()
       },
       BackgroundColor(Color::srgb(0.8, 0.8, 0.9)),
       PickingBehavior::default(),
   ));
}

fn cleanup_inventory(mut commands: Commands, q: Query<Entity, With<InventoryRoot>>) {
   for e in q.iter() {
       commands.entity(e).despawn_recursive();
   }
}

fn debug_grid_gizmos(
   _gizmos: Gizmos,
   _grid_state: Res<InventoryGridState>,
) {
}
