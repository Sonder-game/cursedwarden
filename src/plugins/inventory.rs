use bevy::prelude::*;
use bevy::utils::HashMap;
use crate::plugins::core::GameState;
use crate::plugins::items::ItemDatabase;
use rand::seq::SliceRandom;

pub const TILE_SIZE: f32 = 50.0;
pub const TILE_GAP: f32 = 2.0;
pub const PADDING: f32 = 10.0;

pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InventoryGridState>()
           .add_systems(OnEnter(GameState::EveningPhase), (spawn_inventory_ui,))
           .add_systems(Update, (resize_item_system, spawn_random_item_input).run_if(in_state(GameState::EveningPhase)));
    }
}

// Components
#[derive(Component, Debug, Clone, Copy)]
pub struct InventorySlot {
    pub x: i32,
    pub y: i32,
}

#[derive(Component)]
pub struct Item;

#[derive(Component)]
pub struct InventoryGridContainer;

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

impl InventoryGridState {
    pub fn is_valid_placement(
        &self,
        target_x: i32,
        target_y: i32,
        size: ItemSize,
        exclude_entity: Option<Entity>,
    ) -> bool {
        // Check bounds
        if target_x < 0
            || target_y < 0
            || target_x + size.width > self.width
            || target_y + size.height > self.height
        {
            return false;
        }

        // Check collisions
        for dy in 0..size.height {
            for dx in 0..size.width {
                let check_pos = IVec2::new(target_x + dx, target_y + dy);
                if let Some(occupier) = self.cells.get(&check_pos) {
                    if Some(*occupier) != exclude_entity {
                        return false;
                    }
                }
            }
        }

        true
    }
}

// Systems
fn resize_item_system(
    mut q_items: Query<(&mut Node, &ItemSize), (With<Item>, Changed<ItemSize>)>,
) {
    for (mut node, size) in q_items.iter_mut() {
        // Calculation: width = size.width * TILE_SIZE + (size.width - 1) * TILE_GAP
        //              height = size.height * TILE_SIZE + (size.height - 1) * TILE_GAP
        let width = size.width as f32 * TILE_SIZE + (size.width - 1).max(0) as f32 * TILE_GAP;
        let height = size.height as f32 * TILE_SIZE + (size.height - 1).max(0) as f32 * TILE_GAP;

        node.width = Val::Px(width);
        node.height = Val::Px(height);
    }
}

fn spawn_random_item_input(
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    item_db: Res<ItemDatabase>,
    mut grid_state: ResMut<InventoryGridState>,
    q_grid_parent: Query<Entity, (With<Node>, With<InventorySlot>)>, // This is tricky. We need the grid parent entity.
    // Actually, items are children of the grid container (the one with Display::Grid).
    // We need to query for that specific entity.
    // Let's tag the grid container.
    q_grid_container: Query<Entity, With<InventoryGridContainer>>,
) {
    if input.just_pressed(KeyCode::KeyG) {
        if let Some(item_def) = item_db.items.choose(&mut rand::thread_rng()) {
            // Find a valid spot
            let mut placed = false;
            let item_size = ItemSize { width: item_def.width, height: item_def.height };

            for y in 0..grid_state.height {
                for x in 0..grid_state.width {
                    if grid_state.is_valid_placement(x, y, item_size, None) {
                        // Place item
                        if let Ok(grid_entity) = q_grid_container.get_single() {
                             let item_entity = commands.spawn((
                                Node {
                                    width: Val::Px(item_def.width as f32 * TILE_SIZE + (item_def.width - 1) as f32 * TILE_GAP),
                                    height: Val::Px(item_def.height as f32 * TILE_SIZE + (item_def.height - 1) as f32 * TILE_GAP),
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(PADDING + x as f32 * (TILE_SIZE + TILE_GAP)),
                                    top: Val::Px(PADDING + y as f32 * (TILE_SIZE + TILE_GAP)),
                                    border: UiRect::all(Val::Px(2.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.5, 0.5, 0.8)), // Default color for generated items
                                BorderColor(Color::WHITE),
                                Item,
                                GridPosition { x, y },
                                item_size,
                            ))
                            .observe(handle_drag_start)
                            .observe(handle_drag)
                            .observe(handle_drag_drop)
                            .id();

                            commands.entity(grid_entity).add_child(item_entity);

                            // Update grid state
                            for dy in 0..item_size.height {
                                for dx in 0..item_size.width {
                                    grid_state.cells.insert(IVec2::new(x + dx, y + dy), item_entity);
                                }
                            }

                            info!("Spawned item: {} at ({}, {})", item_def.name, x, y);
                            placed = true;
                        }
                    }
                    if placed { break; }
                }
                if placed { break; }
            }

            if !placed {
                warn!("No space for item: {}", item_def.name);
            }
        }
    }
}


fn spawn_inventory_ui(mut commands: Commands, mut grid_state: ResMut<InventoryGridState>) {
    // Clear any previous state if needed, but ResMut handles current state

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
        ))
        .with_children(|parent| {
            // Inventory Grid Container
            parent.spawn((
                Node {
                    display: Display::Grid,
                    grid_template_columns: vec![GridTrack::px(TILE_SIZE); grid_state.width as usize],
                    grid_template_rows: vec![GridTrack::px(TILE_SIZE); grid_state.height as usize],
                    row_gap: Val::Px(TILE_GAP),
                    column_gap: Val::Px(TILE_GAP),
                    padding: UiRect::all(Val::Px(PADDING)),
                    // Ensure relative positioning context for children (items)
                    position_type: PositionType::Relative,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
                InventoryGridContainer,
            ))
            .with_children(|grid_parent| {
                // Spawn Slots
                for y in 0..grid_state.height {
                    for x in 0..grid_state.width {
                       let slot_entity = grid_parent.spawn((
                            Node {
                                width: Val::Px(TILE_SIZE),
                                height: Val::Px(TILE_SIZE),
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
                            BorderColor(Color::BLACK),
                            InventorySlot { x, y },
                        )).id();

                        // We could populate grid_state here if we want to track slots,
                        // but usually grid_state tracks items.
                        // However, collision logic needs to know if a slot exists?
                        // Actually, slots are static. grid_state.cells usually tracks ITEMS occupying cells.
                    }
                }

                // Spawn Test Item as CHILD of the Grid Container
                // Initial Position: x=0, y=0 -> Left=PADDING, Top=PADDING
                // 2x1 Item
                let item_entity = grid_parent.spawn((
                    Node {
                        width: Val::Px(2.0 * TILE_SIZE + 1.0 * TILE_GAP),
                        height: Val::Px(TILE_SIZE),
                        position_type: PositionType::Absolute,
                        left: Val::Px(PADDING), // Padding
                        top: Val::Px(PADDING),  // Padding
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.8, 0.2, 0.2)),
                    BorderColor(Color::WHITE),
                    Item,
                    GridPosition { x: 0, y: 0 },
                    ItemSize { width: 2, height: 1 },
                ))
                .observe(handle_drag_start)
                .observe(handle_drag)
                .observe(handle_drag_drop)
                .id();

                // Populate grid state with the test item
                grid_state.cells.insert(IVec2::new(0, 0), item_entity);
                grid_state.cells.insert(IVec2::new(1, 0), item_entity);
            });
        });
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
            // Basic validation and collision checking
            if grid_state.is_valid_placement(target_x, target_y, *size, Some(entity)) {
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
                     // Calculation: PADDING + x * (TILE_SIZE + TILE_GAP)
                     let new_left = PADDING + target_x as f32 * (TILE_SIZE + TILE_GAP);
                     let new_top = PADDING + target_y as f32 * (TILE_SIZE + TILE_GAP);

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
