use bevy::prelude::*;
use bevy::utils::HashMap;
use crate::plugins::core::GameState;
use crate::plugins::items::{ItemDatabase, ItemDefinition, SynergyEffect, StatType, ItemType, SynergyVisualType};
use crate::plugins::metagame::{PersistentInventory, SavedItem};
use rand::Rng;

pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InventoryGridState>()
           .init_resource::<PendingCrafts>()
           .add_systems(OnEnter(GameState::EveningPhase), (spawn_inventory_ui, apply_deferred, load_inventory_state, apply_deferred, execute_crafts_system, consume_pending_items).chain())
           .add_systems(OnExit(GameState::EveningPhase), (save_inventory_state, cleanup_inventory_ui).chain())
           .add_systems(Update, (
               resize_item_system,
               debug_spawn_item_system,
               rotate_item_input_system,
               synergy_system,
               visualize_synergy_system,
               update_inventory_slots,
               update_drag_ghost_system, // Ghost Step 7
               draw_inventory_links_system, // Links Step 4
               check_recipes_system, // Crafting Step 4
           ).run_if(in_state(GameState::EveningPhase)))
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

#[derive(Component, Default, Debug)]
pub struct ActiveSynergies {
    pub bonuses: Vec<(StatType, f32)>,
}

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
   // Tracks bags: Entity -> (Position, Rotation, Definition)
   pub bags: HashMap<Entity, (IVec2, u8, ItemDefinition)>,
   pub width: i32,
   pub height: i32,
}

#[derive(Resource, Default)]
pub struct PendingCrafts {
    pub recipes_to_execute: Vec<PendingCraft>,
}

#[derive(Debug, Clone)]
pub struct PendingCraft {
    pub result_id: String,
    pub ingredients: Vec<Entity>,
}

impl Default for InventoryGridState {
    fn default() -> Self {
        // Start empty. Grid is populated by Bags.
        Self {
            grid: HashMap::new(),
            bags: HashMap::new(),
            width: 12, // Larger bounds to allow expansion
            height: 12,
        }
    }
}

pub struct SimulatedItem {
    pub entity_id: Entity,
    pub def: ItemDefinition,
    pub grid_pos: GridPosition,
    pub rotation: ItemRotation,
}

impl InventoryGridState {
    // Helper to reconstruct grid from persistence for offline calculations
    pub fn from_persistent(
        inventory: &PersistentInventory,
        item_db: &ItemDatabase,
    ) -> (Self, Vec<SimulatedItem>) {
        let mut state = Self::default();
        let mut simulated_items = Vec::new();

        // Pass 1: Place Bags
        for (i, saved_item) in inventory.items.iter().enumerate() {
             if let Some(def) = item_db.items.get(&saved_item.item_id) {
                 if def.item_type == ItemType::Bag {
                     let entity_id = Entity::from_raw(i as u32);
                     let pos = IVec2::new(saved_item.grid_x, saved_item.grid_y);
                     let rot = saved_item.rotation;

                     state.bags.insert(entity_id, (pos, rot, def.clone()));
                 }
             }
        }
        state.recalculate_grid();

        // Pass 2: Place Items
        for (i, saved_item) in inventory.items.iter().enumerate() {
            if let Some(def) = item_db.items.get(&saved_item.item_id) {
                if def.item_type != ItemType::Bag {
                    let entity_id = Entity::from_raw(i as u32); // Pseudo-entity
                    let pos = IVec2::new(saved_item.grid_x, saved_item.grid_y);
                    let rot = saved_item.rotation;

                    // Create simulation wrapper
                    simulated_items.push(SimulatedItem {
                        entity_id,
                        def: def.clone(),
                        grid_pos: GridPosition { x: pos.x, y: pos.y },
                        rotation: ItemRotation { value: rot },
                    });

                    // Populate grid
                    let rotated_shape = Self::get_rotated_shape(&def.shape, rot);
                    for offset in rotated_shape {
                        let cell_pos = pos + offset;
                        // Note: We blindly overwrite here, assuming persistence is valid
                        // In a real scenario, we might want to check bounds again
                        if let Some(cell) = state.grid.get_mut(&cell_pos) {
                            cell.state = CellState::Occupied(entity_id);
                        }
                    }
                }
            }
        }

        (state, simulated_items)
    }

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

    // Rebuilds grid slots based on placed bags.
    // Call this whenever a Bag is added, removed, or moved.
    pub fn recalculate_grid(&mut self) {
        // 1. Clear current slots
        self.grid.clear();

        // 2. Add slots from bags
        for (entity, (pos, rot, def)) in &self.bags {
            let shape = Self::get_rotated_shape(&def.shape, *rot);
            for offset in shape {
                let cell_pos = *pos + offset;
                // If overlap, last bag wins (or we should prevent overlap)
                // We insert Free state initially
                self.grid.insert(cell_pos, Cell { state: CellState::Free });
            }
        }
    }

    pub fn can_place_bag(&self, bag_shape: &Vec<IVec2>, pos: IVec2, rotation_step: u8, exclude_entity: Option<Entity>) -> bool {
        let rotated_shape = Self::get_rotated_shape(bag_shape, rotation_step);

        for offset in &rotated_shape {
            let target_pos = pos + *offset;

            // Bags must not overlap OTHER bags
            // We check if any EXISTING bag covers this position
             for (entity, (b_pos, b_rot, b_def)) in &self.bags {
                 if Some(*entity) == exclude_entity { continue; }

                 let b_shape = Self::get_rotated_shape(&b_def.shape, *b_rot);
                 for b_offset in b_shape {
                     if *b_pos + b_offset == target_pos {
                         return false; // Overlap
                     }
                 }
             }
        }

        // Bags must be adjacent to at least one other bag (if not the first one)
        // This is a common rule in Backpack Battles.
        if self.bags.is_empty() { return true; }
        if let Some(exclude) = exclude_entity {
            if self.bags.len() == 1 && self.bags.contains_key(&exclude) {
                return true; // Moving the only bag
            }
        }

        let mut adjacent = false;
        for offset in &rotated_shape {
             let target_pos = pos + *offset;
             let neighbors = [
                 IVec2::new(1, 0), IVec2::new(-1, 0), IVec2::new(0, 1), IVec2::new(0, -1)
             ];
             for n in neighbors {
                 let check_pos = target_pos + n;
                 // Check if check_pos is inside ANY other bag
                 for (entity, (b_pos, b_rot, b_def)) in &self.bags {
                     if Some(*entity) == exclude_entity { continue; }
                     let b_shape = Self::get_rotated_shape(&b_def.shape, *b_rot);
                     for b_offset in b_shape {
                         if *b_pos + b_offset == check_pos {
                             adjacent = true;
                             break;
                         }
                     }
                     if adjacent { break; }
                 }
                 if adjacent { break; }
             }
             if adjacent { break; }
        }

        adjacent
    }

    // New validation function
    pub fn can_place_item(&self, item_shape: &Vec<IVec2>, pos: IVec2, rotation_step: u8, exclude_entity: Option<Entity>) -> bool {
        let rotated_shape = Self::get_rotated_shape(item_shape, rotation_step);

        for offset in rotated_shape {
            let target_pos = pos + offset;

            // Check if cell exists (is valid slot provided by a bag)
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

pub struct CombatStats {
    pub attack: f32,
    pub defense: f32,
    pub speed: f32,
    pub health: f32,
    pub combat_entities: Vec<CombatEntitySnapshot>,
}

#[derive(Debug, Clone)]
pub struct CombatEntitySnapshot {
    pub item_id: String,
    pub final_stats: HashMap<StatType, f32>,
    pub cooldown: f32,
    pub stamina_cost: f32,
    pub accuracy: f32,
}

// Helper to calculate active synergies "offline" (without ECS queries)
pub fn calculate_active_synergies(
    grid_state: &InventoryGridState,
    items: &Vec<SimulatedItem>,
) -> HashMap<Entity, Vec<(StatType, f32)>> {
    let mut pending_bonuses: HashMap<Entity, Vec<(StatType, f32)>> = HashMap::new();

    // Create a quick lookup for item definitions by entity
    let item_lookup: HashMap<Entity, &ItemDefinition> = items.iter().map(|it| (it.entity_id, &it.def)).collect();

    for item in items {
        if item.def.synergies.is_empty() { continue; }

        for synergy in &item.def.synergies {
             // Rotate offset
             let rotated_offset_vec = InventoryGridState::get_rotated_shape(&vec![synergy.offset], item.rotation.value);
             if rotated_offset_vec.is_empty() { continue; }
             let rotated_offset = rotated_offset_vec[0];

             let target_pos = IVec2::new(item.grid_pos.x, item.grid_pos.y) + rotated_offset;

             // Check grid
             if let Some(cell) = grid_state.grid.get(&target_pos) {
                 if let CellState::Occupied(target_entity) = cell.state {
                      // Check target tags
                      if let Some(target_def) = item_lookup.get(&target_entity) {
                          // Check if target has ANY required tag
                          let has_tag = synergy.target_tags.iter().any(|req| target_def.tags.contains(req));

                          if has_tag {
                              match synergy.effect {
                                  SynergyEffect::BuffTarget { stat, value } => {
                                      pending_bonuses.entry(target_entity).or_default().push((stat, value));
                                  },
                                  SynergyEffect::BuffSelf { stat, value } => {
                                      pending_bonuses.entry(item.entity_id).or_default().push((stat, value));
                                  }
                              }
                          }
                      }
                 }
             }
        }
    }

    pending_bonuses
}

pub fn calculate_combat_stats(
    inventory: &PersistentInventory,
    item_db: &ItemDatabase,
) -> CombatStats {
    let mut stats = CombatStats {
        attack: 0.0,
        defense: 0.0,
        speed: 0.0,
        health: 0.0,
        combat_entities: Vec::new(),
    };

    // 1. Reconstruct Grid State
    let (grid_state, simulated_items) = InventoryGridState::from_persistent(inventory, item_db);

    // 2. Calculate Synergies
    let active_bonuses = calculate_active_synergies(&grid_state, &simulated_items);

    // 3. Aggregate Stats
    for item in &simulated_items {
        let mut item_attack = item.def.attack;
        let mut item_defense = item.def.defense;
        let mut item_speed = item.def.speed;

        // Apply bonuses
        if let Some(bonuses) = active_bonuses.get(&item.entity_id) {
            for (stat, val) in bonuses {
                match stat {
                    StatType::Attack => item_attack += val,
                    StatType::Defense => item_defense += val,
                    StatType::Speed => item_speed += val,
                    _ => {}
                }
            }
        }

        // Aggregate to global stats
        stats.attack += item_attack;
        stats.defense += item_defense;
        stats.speed += item_speed;
        // stats.health += item.def.health;

        // Create snapshot for BattleBridge
        let mut final_stats = HashMap::new();
        final_stats.insert(StatType::Attack, item_attack);
        final_stats.insert(StatType::Defense, item_defense);
        final_stats.insert(StatType::Speed, item_speed);

        stats.combat_entities.push(CombatEntitySnapshot {
            item_id: item.def.id.clone(),
            final_stats,
            cooldown: (10.0 - item_speed).max(1.0), // Placeholder cooldown formula
            stamina_cost: 1.0, // Placeholder
            accuracy: 100.0, // Placeholder
        });
    }

    stats
}

// Systems
fn visualize_synergy_system(
    mut q_items: Query<(&ActiveSynergies, &mut BorderColor), Changed<ActiveSynergies>>,
) {
    for (active, mut border) in q_items.iter_mut() {
        if !active.bonuses.is_empty() {
             *border = BorderColor(Color::srgb(1.0, 0.84, 0.0)); // Gold
        } else {
             *border = BorderColor(Color::WHITE);
        }
    }
}

fn synergy_system(
    mut q_items: Query<(Entity, &GridPosition, &ItemRotation, &ItemDefinition, &mut ActiveSynergies)>,
    grid_state: Res<InventoryGridState>,
    q_tags: Query<&ItemDefinition>,
) {
    // 1. Reset all active synergies
    for (_, _, _, _, mut active) in q_items.iter_mut() {
        active.bonuses.clear();
    }

    let mut pending_bonuses: HashMap<Entity, Vec<(StatType, f32)>> = HashMap::new();

    // Read-only pass to find matches
    for (entity, pos, rot, def, _) in q_items.iter() {
        if def.synergies.is_empty() { continue; }

        for synergy in &def.synergies {
             // Rotate offset
             let rotated_offset_vec = InventoryGridState::get_rotated_shape(&vec![synergy.offset], rot.value);
             if rotated_offset_vec.is_empty() { continue; }
             let rotated_offset = rotated_offset_vec[0];

             let target_pos = IVec2::new(pos.x, pos.y) + rotated_offset;

             // Check grid
             if let Some(cell) = grid_state.grid.get(&target_pos) {
                 if let CellState::Occupied(target_entity) = cell.state {
                      // Check target tags
                      if let Ok(target_def) = q_tags.get(target_entity) {
                          // Check if target has ANY required tag
                          let has_tag = synergy.target_tags.iter().any(|req| target_def.tags.contains(req));

                          if has_tag {
                              match synergy.effect {
                                  SynergyEffect::BuffTarget { stat, value } => {
                                      pending_bonuses.entry(target_entity).or_default().push((stat, value));
                                  },
                                  SynergyEffect::BuffSelf { stat, value } => {
                                      pending_bonuses.entry(entity).or_default().push((stat, value));
                                  }
                              }
                          }
                      }
                 }
             }
        }
    }

    // Write pass
    for (entity, _, _, _, mut active) in q_items.iter_mut() {
        if let Some(bonuses) = pending_bonuses.get(&entity) {
            for (stat, val) in bonuses {
                active.bonuses.push((*stat, *val));
            }
        }
    }
}

// Step 7: Ghost Visualization System
fn update_drag_ghost_system(
    mut q_slots: Query<(&InventorySlot, &mut BackgroundColor)>,
    q_dragged: Query<(Entity, &Node, &ItemRotation, &ItemDefinition), With<DragOriginalPosition>>,
    grid_state: Res<InventoryGridState>,
) {
    // 1. Reset Colors to Default
    for (slot, mut bg_color) in q_slots.iter_mut() {
        // Only valid slots exist as entities
        *bg_color = BackgroundColor(Color::srgb(0.3, 0.3, 0.3));
    }

    // 2. If Dragging, Color overlay
    if let Ok((entity, node, rotation, def)) = q_dragged.get_single() {
         let mut left_val = 0.0;
         let mut top_val = 0.0;
         if let Val::Px(l) = node.left { left_val = l; }
         if let Val::Px(t) = node.top { top_val = t; }

         let padding = 10.0;
         let stride = 52.0;

         let (min_x, min_y, _, _) = InventoryGridState::calculate_bounding_box(&def.shape, rotation.value);

         let estimated_pivot_x = ((left_val - padding) / stride).round() as i32 - min_x;
         let estimated_pivot_y = ((top_val - padding) / stride).round() as i32 - min_y;

         let target_pos = IVec2::new(estimated_pivot_x, estimated_pivot_y);
         let is_bag = def.item_type == ItemType::Bag;

         // Check validity
         let is_valid = if is_bag {
             grid_state.can_place_bag(&def.shape, target_pos, rotation.value, Some(entity))
         } else {
             grid_state.can_place_item(&def.shape, target_pos, rotation.value, Some(entity))
         };

         let highlight_color = if is_valid {
             Color::srgba(0.0, 1.0, 0.0, 0.3) // Green
         } else {
             Color::srgba(1.0, 0.0, 0.0, 0.3) // Red
         };

         let rotated_shape = InventoryGridState::get_rotated_shape(&def.shape, rotation.value);

         // Apply color to target slots
         for offset in rotated_shape {
             let slot_pos = target_pos + offset;
             for (slot, mut bg_color) in q_slots.iter_mut() {
                 if slot.x == slot_pos.x && slot.y == slot_pos.y {
                     *bg_color = BackgroundColor(highlight_color);
                 }
             }
         }
    }
}

// Step 4: Crafting & Synergy Lines Visualization
fn draw_inventory_links_system(
    mut gizmos: Gizmos,
    q_items: Query<(Entity, &GridPosition, &ItemRotation, &ItemDefinition)>,
    grid_state: Res<InventoryGridState>,
    pending_crafts: Res<PendingCrafts>,
) {
    let slot_size = 52.0;
    let offset_x = 10.0 + 25.0; // Padding + Half Slot
    let offset_y = 10.0 + 25.0;

    let to_screen = |pos: IVec2| -> Vec2 {
        Vec2::new(
             offset_x + pos.x as f32 * slot_size,
             offset_y + pos.y as f32 * slot_size
        )
    };

    // 1. Draw Synergy Lines
    for (entity, pos, rot, def) in q_items.iter() {
        if def.synergies.is_empty() { continue; }

        for synergy in &def.synergies {
             let rotated_offset_vec = InventoryGridState::get_rotated_shape(&vec![synergy.offset], rot.value);
             if rotated_offset_vec.is_empty() { continue; }
             let rotated_offset = rotated_offset_vec[0];

             let target_pos = IVec2::new(pos.x, pos.y) + rotated_offset;

             if let Some(cell) = grid_state.grid.get(&target_pos) {
                 if let CellState::Occupied(target_entity) = cell.state {
                      // Avoid self-check if somehow mapped
                      if target_entity == entity { continue; }

                      if let Ok((_, _, _, target_def)) = q_items.get(target_entity) {
                           if synergy.target_tags.iter().any(|req| target_def.tags.contains(req)) {
                               // Match! Draw Line.
                               let start = to_screen(IVec2::new(pos.x, pos.y));
                               let end = to_screen(target_pos);

                               // Use different color for Star/Diamond if needed
                               let color = match synergy.visual_type {
                                   SynergyVisualType::Star => Color::srgba(1.0, 0.8, 0.2, 0.8), // Gold/Orange
                                   SynergyVisualType::Diamond => Color::srgba(0.2, 0.8, 1.0, 0.8), // Cyan
                                   _ => Color::srgba(1.0, 1.0, 1.0, 0.5),
                               };
                               gizmos.line_2d(start, end, color);
                           }
                      }
                 }
             }
        }
    }

    // 2. Draw Ready Crafting Recipes (Gold Lines from PendingCrafts)
    for craft in &pending_crafts.recipes_to_execute {
        if craft.ingredients.len() >= 2 {
            // Draw lines between ingredients
            // For 2 items: just one line. For 3+: line to first? or chain?
            // Backpack battles usually connects neighbors.

            let mut positions = Vec::new();
            for &entity in &craft.ingredients {
                if let Ok((_, pos, _, _)) = q_items.get(entity) {
                    positions.push(to_screen(IVec2::new(pos.x, pos.y)));
                }
            }

            // Draw line between 0 and 1
            if positions.len() >= 2 {
                 gizmos.line_2d(positions[0], positions[1], Color::srgba(1.0, 0.84, 0.0, 1.0)); // Thick Gold
                 // Optional: Draw 'pulse' or thickness if Gizmos supported it
            }
        }
    }
}

// Step 4: Logic - Check Recipes and populate PendingCrafts
fn check_recipes_system(
    mut pending_crafts: ResMut<PendingCrafts>,
    q_items: Query<(Entity, &GridPosition, &ItemDefinition)>,
    grid_state: Res<InventoryGridState>,
    item_db: Res<ItemDatabase>,
) {
    // Only run occasionally? Or every frame is fine for prototype.
    pending_crafts.recipes_to_execute.clear();

    // Naive DFS/BFS to find connected components matching recipes is hard.
    // Simplified: Check strict adjacency for 2-ingredient recipes (most common).

    // Track used entities to avoid double counting
    let mut used_entities: Vec<Entity> = Vec::new();

    for recipe in &item_db.recipes {
        if recipe.ingredients.len() != 2 { continue; } // Handle 2-part recipes first

        let item1_id = &recipe.ingredients[0];
        let item2_id = &recipe.ingredients[1];

        // Find all item1s
        for (e1, pos1, def1) in q_items.iter() {
            if used_entities.contains(&e1) { continue; }
            if &def1.id != item1_id { continue; }

            // Check neighbors for item2
            let neighbors = [
                IVec2::new(1, 0), IVec2::new(-1, 0), IVec2::new(0, 1), IVec2::new(0, -1)
            ];

            for n in neighbors {
                let check_pos = IVec2::new(pos1.x, pos1.y) + n;
                if let Some(cell) = grid_state.grid.get(&check_pos) {
                    if let CellState::Occupied(e2) = cell.state {
                         if used_entities.contains(&e2) { continue; }
                         if e1 == e2 { continue; }

                         if let Ok((_, _, def2)) = q_items.get(e2) {
                             if &def2.id == item2_id {
                                 // Found a match!
                                 pending_crafts.recipes_to_execute.push(PendingCraft {
                                     result_id: recipe.result.clone(),
                                     ingredients: vec![e1, e2],
                                 });
                                 used_entities.push(e1);
                                 used_entities.push(e2);
                                 break;
                             }
                         }
                    }
                }
            }
        }
    }
}

// Execute Crafts (OnEnter Evening)
fn execute_crafts_system(
    mut commands: Commands,
    mut pending_crafts: ResMut<PendingCrafts>,
    mut grid_state: ResMut<InventoryGridState>,
    item_db: Res<ItemDatabase>,
    q_container: Query<Entity, With<InventoryGridContainer>>,
    q_pos: Query<&GridPosition>,
) {
    if let Ok(container) = q_container.get_single() {
        for craft in &pending_crafts.recipes_to_execute {
             // 1. Remove ingredients
             // We need to pick a position for the result. Use the first ingredient's pos.
             let mut result_pos = IVec2::ZERO;
             if let Ok(pos) = q_pos.get(craft.ingredients[0]) {
                 result_pos = IVec2::new(pos.x, pos.y);
             }

             for entity in &craft.ingredients {
                 // Clear from grid
                 // Manual clear to ensure space is free for result in THIS frame
                 // (despawn is deferred)

                 let mut cells_to_clear = Vec::new();
                 for (pos, cell) in grid_state.grid.iter() {
                     if let CellState::Occupied(occupier) = cell.state {
                         if occupier == *entity {
                             cells_to_clear.push(*pos);
                         }
                     }
                 }
                 for pos in cells_to_clear {
                     if let Some(cell) = grid_state.grid.get_mut(&pos) {
                         cell.state = CellState::Free;
                     }
                 }

                 // Remove entity
                 commands.entity(*entity).despawn_recursive();
             }

             // 2. Spawn result
             if let Some(def) = item_db.items.get(&craft.result_id) {
                 // Try place at result_pos, if fails, find free spot
                 if grid_state.can_place_item(&def.shape, result_pos, 0, None) {
                      spawn_item_entity(&mut commands, container, def, result_pos, 0, &mut grid_state);
                      info!("Crafted {}!", def.name);
                 } else if let Some(free_pos) = grid_state.find_free_spot(def) {
                      spawn_item_entity(&mut commands, container, def, free_pos, 0, &mut grid_state);
                      info!("Crafted {} (moved)!", def.name);
                 } else {
                      warn!("Crafted {} but no space found! (Items lost)", def.name);
                 }
             }
        }
    }
    // Clear pending
    pending_crafts.recipes_to_execute.clear();
    // Rebuild grid to be safe
    grid_state.recalculate_grid();
}

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
            // IMPORTANT: We cannot use standard Grid Layout for non-rectangular shapes easily
            // if we want to visualize "holes".
            // However, we can keep the 12x12 container but only spawn visible children in valid slots.
            parent.spawn((
                InventoryGridContainer,
                Node {
                    display: Display::Grid,
                    // Use standard 12x12 grid template for layout stability
                    grid_template_columns: vec![GridTrack::px(50.0); grid_state.width as usize],
                    grid_template_rows: vec![GridTrack::px(50.0); grid_state.height as usize],
                    row_gap: Val::Px(2.0),
                    column_gap: Val::Px(2.0),
                    padding: UiRect::all(Val::Px(10.0)),
                    // Ensure relative positioning context for children (items)
                    position_type: PositionType::Relative,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.1)), // Semi-transparent bg
            ));
        });

    // Slots are now spawned by update_inventory_slots
}

fn update_inventory_slots(
    mut commands: Commands,
    grid_state: Res<InventoryGridState>,
    q_container: Query<Entity, With<InventoryGridContainer>>,
    q_slots: Query<Entity, With<InventorySlot>>,
) {
    if !grid_state.is_changed() { return; }

    // Clear existing slots
    for e in q_slots.iter() {
        commands.entity(e).despawn_recursive();
    }

    // Spawn new slots
    if let Ok(container) = q_container.get_single() {
        commands.entity(container).with_children(|grid_parent| {
            for y in 0..grid_state.height {
                for x in 0..grid_state.width {
                    let pos = IVec2::new(x, y);
                    let is_valid = grid_state.grid.contains_key(&pos);

                    // We spawn a node for EVERY slot to maintain grid structure (CSS Grid cell alignment)
                    // But we make invalid slots invisible/transparent

                    let bg_color = if is_valid {
                        Color::srgb(0.3, 0.3, 0.3)
                    } else {
                        Color::NONE // Invisible
                    };

                    let border_color = if is_valid {
                        Color::BLACK
                    } else {
                        Color::NONE
                    };

                    // Only render valid slots with distinct style
                    grid_parent.spawn((
                        Node {
                            width: Val::Px(50.0),
                            height: Val::Px(50.0),
                            border: if is_valid { UiRect::all(Val::Px(1.0)) } else { UiRect::default() },
                            ..default()
                        },
                        BackgroundColor(bg_color),
                        BorderColor(border_color),
                        InventorySlot { x, y },
                    ));
                }
            }
        });
    }
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
    q_items: Query<(&ItemDefinition, &GridPosition, &ItemRotation), With<Item>>,
) {
    let mut saved_items = Vec::new();
    for (def, pos, rot) in q_items.iter() {
        saved_items.push(SavedItem {
            item_id: def.id.clone(),
            grid_x: pos.x,
            grid_y: pos.y,
            rotation: rot.value,
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

        // Pass 1: Bags (Critical to establish grid)
        for saved_item in &persistent_inventory.items {
            if let Some(def) = item_db.items.get(&saved_item.item_id) {
                if def.item_type == ItemType::Bag {
                    let pos = IVec2::new(saved_item.grid_x, saved_item.grid_y);
                    // Force spawn bag without validation (assumed valid from save),
                    // or validate if we want to be safe.
                    // For Bags, we don't check 'can_place_item' (which checks for slots),
                    // we check 'can_place_bag'.
                    if grid_state.can_place_bag(&def.shape, pos, saved_item.rotation, None) {
                        spawn_item_entity(
                            &mut commands,
                            container,
                            def,
                            pos,
                            saved_item.rotation,
                            &mut grid_state
                        );
                    } else {
                         warn!("Could not restore bag {} at {:?}: Invalid placement", def.name, pos);
                    }
                }
            }
        }

        // Pass 2: Items
        for saved_item in &persistent_inventory.items {
            if let Some(def) = item_db.items.get(&saved_item.item_id) {
                 if def.item_type != ItemType::Bag {
                     let pos = IVec2::new(saved_item.grid_x, saved_item.grid_y);

                     if grid_state.can_place_item(&def.shape, pos, saved_item.rotation, None) {
                         spawn_item_entity(
                             &mut commands,
                             container,
                             def,
                             pos,
                             saved_item.rotation,
                             &mut grid_state
                         );
                     } else {
                         warn!("Could not restore item {} at {:?}: Space occupied", def.name, pos);
                     }
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

                 // If it's a bag, try place bag
                 if def.item_type == ItemType::Bag {
                      warn!("Auto-placing bags from city not fully implemented yet.");
                 } else {
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
pub fn spawn_item_entity(
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

     let is_bag = def.item_type == ItemType::Bag;

     // Bags: Lower Z-Index, Different color
     // Items: Higher Z-Index
     let z_idx = if is_bag { ZIndex(1) } else { ZIndex(10) };
     let color = if is_bag { Color::srgb(0.4, 0.2, 0.1) } else { Color::srgb(0.5, 0.5, 0.8) };
     let border_col = if is_bag { Color::NONE } else { Color::WHITE };

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
        BackgroundColor(color),
        BorderColor(border_col),
        Interaction::default(),
        Item,
        GridPosition { x: pos.x, y: pos.y },
        ItemSize { width: width_slots, height: height_slots },
        ItemRotation { value: rotation },
        ActiveSynergies::default(),
        z_idx,
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

    // Logic Update
    if is_bag {
        // Update Bags Map
        grid_state.bags.insert(item_entity, (pos, rotation, def.clone()));
        // Update Grid Slots (Recalculate all)
        grid_state.recalculate_grid();
    } else {
        // Occupy Grid Slots
        let rotated_shape = InventoryGridState::get_rotated_shape(&def.shape, rotation);
        for offset in rotated_shape {
            let cell_pos = pos + offset;
            if let Some(cell) = grid_state.grid.get_mut(&cell_pos) {
                cell.state = CellState::Occupied(item_entity);
            }
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
    mut q_item: Query<(&mut ZIndex, &mut Node, &mut ItemRotation, &mut ItemSize, &mut GridPosition, &ItemDefinition), (With<Item>, With<DragOriginalPosition>)>,
    q_all_items: Query<(Entity, &GridPosition, &ItemRotation, &ItemDefinition), (With<Item>, Without<DragOriginalPosition>)>,
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

        // Validation Logic Branch
        let mut success = false;

        if def.item_type == ItemType::Bag {
            if grid_state.can_place_bag(&def.shape, target_pos, rotation.value, Some(entity)) {
                // Update Bag List
                grid_state.bags.insert(entity, (target_pos, rotation.value, def.clone()));
                // Recalculate Slots (clears occupancy)
                grid_state.recalculate_grid();

                // Re-register all OTHER items (not the dragged one yet)
                for (other_entity, other_pos, other_rot, other_def) in q_all_items.iter() {
                    // Skip bags in this pass (they don't occupy slots)
                    if other_def.item_type == ItemType::Bag { continue; }

                    let rotated_shape = InventoryGridState::get_rotated_shape(&other_def.shape, other_rot.value);
                    for offset in rotated_shape {
                        let cell_pos = IVec2::new(other_pos.x, other_pos.y) + offset;
                        if let Some(cell) = grid_state.grid.get_mut(&cell_pos) {
                            cell.state = CellState::Occupied(other_entity);
                        }
                    }
                }

                success = true;
            }
        } else {
             // Normal Item
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
                 success = true;
            }
        }

        if success {

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::items::{ItemTag, SynergyDefinition, SynergyEffect, StatType};

    #[test]
    fn test_synergy_calculation() {
        let mut item_db = ItemDatabase::default();

        let sword = ItemDefinition {
            id: "sword".to_string(),
            name: "Sword".to_string(),
            width: 1, height: 1, shape: vec![IVec2::new(0,0)],
            material: crate::plugins::items::MaterialType::Steel,
            item_type: crate::plugins::items::ItemType::Weapon,
            tags: vec![ItemTag::Weapon],
            synergies: vec![],
            attack: 10.0, defense: 0.0, speed: 0.0,
            rarity: crate::plugins::items::ItemRarity::Common,
            price: 10,
        };

        let whetstone = ItemDefinition {
            id: "whetstone".to_string(),
            name: "Stone".to_string(),
            width: 1, height: 1, shape: vec![IVec2::new(0,0)],
            material: crate::plugins::items::MaterialType::Steel,
            item_type: crate::plugins::items::ItemType::Consumable,
            tags: vec![],
            synergies: vec![
                SynergyDefinition {
                    offset: IVec2::new(1, 0),
                    target_tags: vec![ItemTag::Weapon],
                    effect: SynergyEffect::BuffTarget { stat: StatType::Attack, value: 5.0 },
                    visual_type: crate::plugins::items::SynergyVisualType::Star,
                }
            ],
            attack: 0.0, defense: 0.0, speed: 0.0,
            rarity: crate::plugins::items::ItemRarity::Common,
            price: 5,
        };

        // Add "starter_bag" for test context since PersistentInventory now defaults to it
        let starter_bag = ItemDefinition {
            id: "starter_bag".to_string(),
            name: "Starter Bag".to_string(),
            width: 3, height: 3, shape: vec![], // Auto-generated
            material: crate::plugins::items::MaterialType::Flesh,
            item_type: crate::plugins::items::ItemType::Bag,
            tags: vec![], synergies: vec![],
            attack: 0.0, defense: 0.0, speed: 0.0,
            rarity: crate::plugins::items::ItemRarity::Common,
            price: 0,
        };
        // We need to auto-generate shape for starter_bag manually as load_items isn't running
        let mut bag_with_shape = starter_bag.clone();
        for y in 0..3 { for x in 0..3 { bag_with_shape.shape.push(IVec2::new(x,y)); } }

        item_db.items.insert("sword".to_string(), sword);
        item_db.items.insert("whetstone".to_string(), whetstone);
        item_db.items.insert("starter_bag".to_string(), bag_with_shape);

        let mut inv = PersistentInventory::default();
        // Clear default starter bag to ensure we set up test exactly as needed
        inv.items.clear();

        // Place Bag at (2,2) -> Covers (2,2) to (4,4)
        inv.items.push(SavedItem { item_id: "starter_bag".to_string(), grid_x: 2, grid_y: 2, rotation: 0 });

        // Place Items
        // Sword at (3,2) (Inside bag)
        inv.items.push(SavedItem { item_id: "sword".to_string(), grid_x: 3, grid_y: 2, rotation: 0 });
        // Whetstone at (2,2) (Inside bag)
        inv.items.push(SavedItem { item_id: "whetstone".to_string(), grid_x: 2, grid_y: 2, rotation: 0 });

        // Synergy: Whetstone at (2,2) with Offset (1,0) looks at (3,2).
        // (3,2) has Sword. Synergy triggers.

        let stats = calculate_combat_stats(&inv, &item_db);
        assert_eq!(stats.attack, 15.0); // 10 Base + 5 Bonus

        let sword_entity = stats.combat_entities.iter().find(|e| e.item_id == "sword").unwrap();
        assert_eq!(sword_entity.final_stats.get(&StatType::Attack), Some(&15.0));
    }

    #[test]
    fn test_crafting_logic() {
         let mut app = App::new();
         app.add_plugins(MinimalPlugins);
         app.init_resource::<InventoryGridState>();
         app.init_resource::<PendingCrafts>();
         app.init_resource::<ItemDatabase>();

         // Setup DB
         let mut item_db = app.world_mut().resource_mut::<ItemDatabase>();
         item_db.items.insert("ing1".to_string(), ItemDefinition {
             id: "ing1".to_string(), name: "Ing1".to_string(),
             width: 1, height: 1, shape: vec![IVec2::new(0,0)],
             material: crate::plugins::items::MaterialType::Steel,
             item_type: crate::plugins::items::ItemType::Weapon,
             tags: vec![], synergies: vec![],
             attack: 0.0, defense: 0.0, speed: 0.0, rarity: crate::plugins::items::ItemRarity::Common, price: 0
         });
         item_db.items.insert("ing2".to_string(), ItemDefinition {
             id: "ing2".to_string(), name: "Ing2".to_string(),
             width: 1, height: 1, shape: vec![IVec2::new(0,0)],
             material: crate::plugins::items::MaterialType::Steel,
             item_type: crate::plugins::items::ItemType::Weapon,
             tags: vec![], synergies: vec![],
             attack: 0.0, defense: 0.0, speed: 0.0, rarity: crate::plugins::items::ItemRarity::Common, price: 0
         });
         item_db.items.insert("result".to_string(), ItemDefinition {
             id: "result".to_string(), name: "Result".to_string(),
             width: 1, height: 1, shape: vec![IVec2::new(0,0)],
             material: crate::plugins::items::MaterialType::Steel,
             item_type: crate::plugins::items::ItemType::Weapon,
             tags: vec![], synergies: vec![],
             attack: 0.0, defense: 0.0, speed: 0.0, rarity: crate::plugins::items::ItemRarity::Common, price: 0
         });
         item_db.recipes.push(crate::plugins::items::RecipeDefinition {
             ingredients: vec!["ing1".to_string(), "ing2".to_string()],
             result: "result".to_string(),
             catalysts: vec![],
         });

         // Spawn Items manually (mocking ECS)
         let e1 = app.world_mut().spawn((
             Item,
             GridPosition { x: 0, y: 0 },
             ItemDefinition { id: "ing1".to_string(), ..default() } // Simplified
         )).id();

         let e2 = app.world_mut().spawn((
             Item,
             GridPosition { x: 1, y: 0 },
             ItemDefinition { id: "ing2".to_string(), ..default() }
         )).id();

         // Update Grid State manually
         let mut grid = app.world_mut().resource_mut::<InventoryGridState>();
         grid.grid.insert(IVec2::new(0,0), Cell { state: CellState::Occupied(e1) });
         grid.grid.insert(IVec2::new(1,0), Cell { state: CellState::Occupied(e2) });

         // Run Check System
         app.add_systems(Update, check_recipes_system);
         app.update();

         // Check Pending
         let pending = app.world().resource::<PendingCrafts>();
         assert_eq!(pending.recipes_to_execute.len(), 1);
         assert_eq!(pending.recipes_to_execute[0].result_id, "result");
    }
}
