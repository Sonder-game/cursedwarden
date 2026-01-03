use bevy::prelude::*;
use bevy::utils::HashMap;
use crate::plugins::core::GameState;

pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InventoryGridState>()
           .add_systems(OnEnter(GameState::EveningPhase), (spawn_inventory_ui,));
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

// Systems
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
            ))
            .with_children(|grid_parent| {
                // Spawn Slots
                for y in 0..grid_state.height {
                    for x in 0..grid_state.width {
                       let slot_entity = grid_parent.spawn((
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
             // Basic validation: Check bounds
             if target_x >= 0 && target_y >= 0 &&
                target_x + size.width <= grid_state.width &&
                target_y + size.height <= grid_state.height
             {
                 // Check collisions
                 let mut collision = false;
                 // We need to check all cells the item would occupy
                 for dy in 0..size.height {
                     for dx in 0..size.width {
                         let check_pos = IVec2::new(target_x + dx, target_y + dy);
                         if let Some(occupier) = grid_state.cells.get(&check_pos) {
                             if *occupier != entity {
                                 collision = true;
                                 break;
                             }
                         }
                     }
                     if collision { break; }
                 }

                 if !collision {
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
