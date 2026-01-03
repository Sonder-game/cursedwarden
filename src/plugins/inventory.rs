use bevy::prelude::*;
use bevy::utils::HashMap;
use crate::plugins::core::GameState;
use crate::plugins::items::{ItemDatabase, ItemDefinition};
use crate::plugins::metagame::SavedItem;
use rand::Rng;

pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InventoryGridState>()
           .init_resource::<PersistentInventory>()
           .init_resource::<IncomingItems>()
           .add_systems(OnEnter(GameState::EveningPhase), (load_inventory_state, spawn_inventory_ui, populate_inventory_system).chain())
           .add_systems(OnExit(GameState::EveningPhase), (save_inventory_state, teardown_inventory_ui))
           .add_systems(Update, (resize_item_system, debug_spawn_item_system, inventory_ui_interactions).run_if(in_state(GameState::EveningPhase)))
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
pub struct InventoryUiRoot;

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

#[derive(Resource, Default)]
pub struct PersistentInventory {
    pub items: Vec<SavedItem>,
}

#[derive(Resource, Default)]
pub struct IncomingItems {
    pub items: Vec<ItemDefinition>,
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

fn save_inventory_state(
    mut persistent: ResMut<PersistentInventory>,
    q_items: Query<(&ItemDefinition, &GridPosition), With<Item>>,
) {
    persistent.items.clear();
    for (def, pos) in q_items.iter() {
        persistent.items.push(SavedItem {
            item_id: def.id.clone(),
            grid_x: pos.x,
            grid_y: pos.y,
        });
    }
    info!("Saved {} items to PersistentInventory", persistent.items.len());
}

fn load_inventory_state(
    mut grid_state: ResMut<InventoryGridState>,
) {
    // Clear current grid state (entities are cleared by teardown/despawn, but map needs reset)
    grid_state.cells.clear();
}

// Split logic
fn populate_inventory_system(
    mut commands: Commands,
    mut grid_state: ResMut<InventoryGridState>,
    mut incoming: ResMut<IncomingItems>,
    persistent: Res<PersistentInventory>,
    item_db: Res<ItemDatabase>,
    q_container: Query<Entity, With<InventoryGridContainer>>,
) {
    if let Ok(container) = q_container.get_single() {
        // 1. Restore persistent items
        for saved_item in &persistent.items {
             if let Some(def) = item_db.items.get(&saved_item.item_id) {
                 spawn_item_entity(
                     &mut commands,
                     &mut grid_state,
                     container,
                     def.clone(),
                     IVec2::new(saved_item.grid_x, saved_item.grid_y)
                 );
             }
        }

        // 2. Add incoming items
        for def in incoming.items.drain(..) {
            let size = ItemSize { width: def.width as i32, height: def.height as i32 };
            if let Some(pos) = grid_state.find_free_spot(size) {
                spawn_item_entity(
                     &mut commands,
                     &mut grid_state,
                     container,
                     def.clone(),
                     pos
                );
            } else {
                warn!("No space for incoming item {}", def.name);
                // Could push back to incoming or drop? For now drop.
            }
        }
    }
}

// Helper
fn spawn_item_entity(
    commands: &mut Commands,
    grid_state: &mut InventoryGridState,
    container: Entity,
    def: ItemDefinition,
    pos: IVec2,
) -> Entity {
     let size = ItemSize { width: def.width as i32, height: def.height as i32 };

     // Visuals
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
         ));
    })
    .id();

    // Trigger event to attach drag observers
    commands.trigger(ItemSpawnedEvent(item_entity));

    // Add to grid state
    for dy in 0..size.height {
        for dx in 0..size.width {
            grid_state.cells.insert(IVec2::new(pos.x + dx, pos.y + dy), item_entity);
        }
    }

    commands.entity(container).add_child(item_entity);
    item_entity
}


fn spawn_inventory_ui(mut commands: Commands, grid_state: ResMut<InventoryGridState>) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                display: Display::Flex,
                flex_direction: FlexDirection::Column, // Changed for Button
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
                    position_type: PositionType::Relative,
                    margin: UiRect::bottom(Val::Px(20.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
            ))
            .with_children(|grid_parent| {
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

            // Start Night Button
            parent.spawn((
                Button,
                Node {
                    width: Val::Px(200.0),
                    height: Val::Px(50.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.5, 0.0, 0.0)),
                BorderColor(Color::WHITE),
            ))
            .with_children(|p| {
                p.spawn((
                    Text::new("START NIGHT (Combat)"),
                    TextFont { font_size: 20.0, ..default() },
                    TextColor(Color::WHITE),
                ));
            });
        });
}

fn teardown_inventory_ui(
    mut commands: Commands,
    q_root: Query<Entity, With<InventoryUiRoot>>,
) {
    for e in q_root.iter() {
        commands.entity(e).despawn_recursive();
    }
}

fn inventory_ui_interactions(
    mut next_state: ResMut<NextState<GameState>>,
    mut q_interaction: Query<(&Interaction, &BackgroundColor), (Changed<Interaction>, With<Button>)>,
) {
    for (interaction, _color) in q_interaction.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                next_state.set(GameState::NightPhase);
            }
            _ => {}
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
                         &mut grid_state,
                         container,
                         def.clone(),
                         pos
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
        .observe(handle_drag_drop);
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

fn handle_drag_drop(
    trigger: Trigger<Pointer<DragDrop>>,
    mut commands: Commands,
    mut q_item: Query<(&mut ZIndex, &mut Node, &ItemSize, &mut GridPosition), With<Item>>,
    q_slots: Query<&InventorySlot>,
    q_original: Query<&DragOriginalPosition>,
    mut grid_state: ResMut<InventoryGridState>,
) {
    let entity = trigger.entity();
    let event = trigger.event();

    // Check if dropped on a slot
    let mut dropped_on_slot = false;
    let mut target_x = 0;
    let mut target_y = 0;

    if let Ok(slot) = q_slots.get(event.target) {
        dropped_on_slot = true;
        target_x = slot.x;
        target_y = slot.y;
    }

    if dropped_on_slot {
        if let Ok((mut z_index, mut node, size, mut grid_pos)) = q_item.get_mut(entity) {
             // Basic validation and collision check
             if grid_state.is_area_free(IVec2::new(target_x, target_y), *size, Some(entity)) {
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

                 // If valid, snap to slot
                 // Assuming 50px slots + 2px gaps + 10px padding.
                 // Calculation: 10 + x * (50 + 2)
                 let new_left = 10.0 + target_x as f32 * 52.0;
                 let new_top = 10.0 + target_y as f32 * 52.0;

                 node.left = Val::Px(new_left);
                 node.top = Val::Px(new_top);

                 // Update logical position
                 grid_pos.x = target_x;
                 grid_pos.y = target_y;

                 // Restore Z-Index (maybe +1 so it sits above grid but not everything)
                 if let Ok(original) = q_original.get(entity) {
                      *z_index = original.z_index;
                 } else {
                      *z_index = ZIndex(0);
                 }

                 commands.entity(entity).remove::<DragOriginalPosition>();
                 return;
             }
        }
    }

    // If invalid or not dropped on slot, revert
    if let Ok(original) = q_original.get(entity) {
        if let Ok((mut z_index, mut node, _, _)) = q_item.get_mut(entity) {
             *z_index = original.z_index;
             node.left = original.left;
             node.top = original.top;
        }
        commands.entity(entity).remove::<DragOriginalPosition>();
    }
}
