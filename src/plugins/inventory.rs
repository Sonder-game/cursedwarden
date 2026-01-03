use bevy::prelude::*;
use bevy::utils::HashMap;
use crate::plugins::core::GameState;
use crate::plugins::items::ItemDatabase;

pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InventoryGridState>()
           .add_systems(OnEnter(GameState::EveningPhase), (spawn_inventory_ui,))
           .add_systems(Update, (resize_item_system, debug_mutation_trigger, debug_spawn_item).run_if(in_state(GameState::EveningPhase)));
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
    pub fn is_area_free(&self, start: IVec2, size: &ItemSize, exclude: Option<Entity>) -> bool {
        if start.x < 0 || start.y < 0 {
            return false;
        }
        if start.x + size.width > self.width || start.y + size.height > self.height {
            return false;
        }

        for dy in 0..size.height {
            for dx in 0..size.width {
                let pos = IVec2::new(start.x + dx, start.y + dy);
                if let Some(occupier) = self.cells.get(&pos) {
                     if let Some(exc) = exclude {
                         if *occupier == exc {
                             continue;
                         }
                     }
                     return false;
                }
            }
        }
        true
    }
}

// Systems
fn resize_item_system(
    mut q_items: Query<(&mut Node, &ItemSize), Changed<ItemSize>>,
) {
    for (mut node, size) in q_items.iter_mut() {
        // Update UI size based on ItemSize
        // 50px per unit + 2px gap per unit-1 + padding/border considerations?
        // Actually, the slot is 50px.
        // Width = (width * 50) + ((width - 1) * 2)
        let width_px = (size.width as f32 * 50.0) + ((size.width as f32 - 1.0).max(0.0) * 2.0);
        let height_px = (size.height as f32 * 50.0) + ((size.height as f32 - 1.0).max(0.0) * 2.0);

        node.width = Val::Px(width_px);
        node.height = Val::Px(height_px);
    }
}

fn debug_mutation_trigger(
    input: Res<ButtonInput<KeyCode>>,
    mut q_items: Query<&mut ItemSize, With<Item>>,
    _grid_state: Res<InventoryGridState>,
) {
    if input.just_pressed(KeyCode::Space) {
        for mut size in q_items.iter_mut() {
            // Simple mutation: try to grow height by 1
            // In a real system, we'd check if we can grow without overlap using is_area_free
            // But for now, let's just trigger the change to test resize_item_system
            if size.height < 3 {
                 size.height += 1;
                 info!("Mutated item to size: {}x{}", size.width, size.height);
            } else {
                 size.height = 1; // Reset
                 info!("Reset item to size: {}x{}", size.width, size.height);
            }
            // Note: We aren't updating grid_state here correctly for the new size/overlap!
            // This is just to test the visual resize.
            // In a real mutation system, we must check is_area_free and update grid_state.
        }
    }
}

fn debug_spawn_item(
    input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut grid_state: ResMut<InventoryGridState>,
    item_db: Res<ItemDatabase>,
    q_grid_node: Query<Entity, With<InventoryGridContainer>>,
) {
    if input.just_pressed(KeyCode::KeyA) {
        // Try to spawn a Steel Sword (1x2)
        if let Some(item_def) = item_db.items.get("steel_sword") {
             let size = ItemSize { width: item_def.width as i32, height: item_def.height as i32 };

             // Simple "Generation" logic: Find first free spot
             for y in 0..grid_state.height {
                 for x in 0..grid_state.width {
                     let pos = IVec2::new(x, y);
                     if grid_state.is_area_free(pos, &size, None) {
                         // Spawn it
                         if let Ok(grid_entity) = q_grid_node.get_single() {
                             let width_px = (size.width as f32 * 50.0) + ((size.width as f32 - 1.0).max(0.0) * 2.0);
                             let height_px = (size.height as f32 * 50.0) + ((size.height as f32 - 1.0).max(0.0) * 2.0);
                             let left_px = 10.0 + x as f32 * 52.0;
                             let top_px = 10.0 + y as f32 * 52.0;

                             let item_entity = commands.spawn((
                                Node {
                                    width: Val::Px(width_px),
                                    height: Val::Px(height_px),
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(left_px),
                                    top: Val::Px(top_px),
                                    border: UiRect::all(Val::Px(2.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.5, 0.5, 0.8)), // Different color for spawned items
                                BorderColor(Color::WHITE),
                                Item,
                                GridPosition { x, y },
                                size,
                            ))
                            .observe(handle_drag_start)
                            .observe(handle_drag)
                            .observe(handle_drag_drop)
                            .set_parent(grid_entity)
                            .id();

                            // Update Grid State
                            for dy in 0..size.height {
                                for dx in 0..size.width {
                                    let p = IVec2::new(x + dx, y + dy);
                                    grid_state.cells.insert(p, item_entity);
                                }
                            }

                            info!("Spawned item at {}, {}", x, y);
                            return;
                         }
                     }
                 }
             }
             warn!("No space for item!");
        }
    }
}

#[derive(Component)]
struct InventoryGridContainer;

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
                InventoryGridContainer, // Marker component
            ))
            .with_children(|grid_parent| {
                // Spawn Slots
                for y in 0..grid_state.height {
                    for x in 0..grid_state.width {
                       let _slot_entity = grid_parent.spawn((
                            Node {
                                width: Val::Px(50.0),
                                height: Val::Px(50.0),
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
                // Initial Position: x=0, y=0 -> Left=10px, Top=10px (padding)
                // 2x1 Item
                let item_entity = grid_parent.spawn((
                    Node {
                        width: Val::Px(100.0 + 2.0), // 2 * 50px + 1 * 2px gap (approx logic)
                        // Actually: 2 slots * 50px + 1 gap * 2px = 102px
                        height: Val::Px(50.0),
                        position_type: PositionType::Absolute,
                        left: Val::Px(10.0), // Padding
                        top: Val::Px(10.0),  // Padding
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
             let target_pos = IVec2::new(target_x, target_y);
             if grid_state.is_area_free(target_pos, size, Some(entity)) {
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
