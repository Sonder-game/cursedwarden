use bevy::prelude::*;
use bevy::utils::HashMap;
use crate::plugins::core::GameState;

pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InventoryGridState>()
           .add_systems(OnEnter(GameState::EveningPhase), spawn_inventory_ui)
           .add_systems(OnEnter(GameState::EveningPhase), spawn_test_item.after(spawn_inventory_ui))
           .add_observer(on_drag_start)
           .add_observer(on_drag)
           .add_observer(on_drop);
    }
}

// Components
#[derive(Component)]
pub struct InventorySlot {
    pub x: i32,
    pub y: i32,
}

#[derive(Component)]
pub struct Item;

#[derive(Component)]
pub struct GridPosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Component)]
pub struct ItemSize {
    pub width: i32,
    pub height: i32,
}

#[derive(Component)]
pub struct OriginalPosition(pub Val, pub Val);

#[derive(Component)]
pub struct GridContainer;

// Resources
#[derive(Resource, Default)]
pub struct InventoryGridState {
   pub cells: HashMap<IVec2, Entity>,
   pub width: i32,
   pub height: i32,
}

// Systems
fn spawn_inventory_ui(mut commands: Commands) {
    // Basic UI setup for inventory
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            display: Display::Flex,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                Node {
                    display: Display::Grid,
                    grid_template_columns: vec![GridTrack::px(50.0); 8],
                    grid_template_rows: vec![GridTrack::px(50.0); 8],
                    row_gap: Val::Px(2.0),
                    column_gap: Val::Px(2.0),
                    padding: UiRect::all(Val::Px(10.0)),
                    ..default()
                },
                GridContainer
            )).with_children(|grid| {
                for y in 0..8 {
                    for x in 0..8 {
                        grid.spawn((
                            Node {
                                width: Val::Px(50.0),
                                height: Val::Px(50.0),
                                border: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                            BorderColor(Color::BLACK),
                            BackgroundColor(Color::srgb(0.5, 0.5, 0.5)),
                            InventorySlot { x, y },
                            PickingBehavior::default(),
                        ));
                    }
                }
            });
        });
}

fn spawn_test_item(mut commands: Commands, grid_query: Query<Entity, With<GridContainer>>) {
    if let Ok(grid_entity) = grid_query.get_single() {
        // Spawn a test item (1x2) as child of GridContainer
        commands.entity(grid_entity).with_children(|parent| {
            parent.spawn((
                Node {
                    width: Val::Px(50.0), // 1 slot wide
                    height: Val::Px(100.0), // 2 slots high
                    position_type: PositionType::Absolute, // To move freely within grid container
                    // Initial position at (0,0) -> padding
                    left: Val::Px(10.0),
                    top: Val::Px(10.0),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.8, 0.2, 0.2)),
                BorderColor(Color::WHITE),
                Item,
                ItemSize { width: 1, height: 2 },
                GridPosition { x: 0, y: 0 }, // Logic position
                PickingBehavior::default(),
                ZIndex(1), // Start with a slightly higher Z-Index than slots
            ));
        });
    }
}

// Observers for Drag and Drop
fn on_drag_start(
    trigger: Trigger<Pointer<DragStart>>,
    mut commands: Commands,
    mut queries: Query<(&mut ZIndex, &Node), With<Item>>
) {
    if let Ok((mut z_index, node)) = queries.get_mut(trigger.entity()) {
        *z_index = ZIndex(100); // Bring to front
        commands.entity(trigger.entity()).insert(OriginalPosition(node.left, node.top));
    }
}

fn on_drag(trigger: Trigger<Pointer<Drag>>, mut queries: Query<&mut Node, With<Item>>) {
    if let Ok(mut node) = queries.get_mut(trigger.entity()) {
        // Simplified handling assuming Val::Px
        let dx = trigger.event().delta.x;
        let dy = trigger.event().delta.y;

        if let Val::Px(current_left) = node.left {
            node.left = Val::Px(current_left + dx);
        }
        if let Val::Px(current_top) = node.top {
            node.top = Val::Px(current_top + dy);
        }
    }
}

fn on_drop(
    trigger: Trigger<Pointer<DragDrop>>,
    mut commands: Commands,
    mut queries: Query<(&mut ZIndex, &mut Node, &OriginalPosition, &mut GridPosition, &ItemSize), With<Item>>,
    // slots: Query<(&GlobalTransform, &InventorySlot)>, // Unused for now
    // In a real app we'd also check InventoryGridState for occupancy
) {
    let dropped_entity = trigger.entity();
    let _event = trigger.event();

    // Logic: Same as before, but now node.left/top are relative to GridContainer.
    // GridContainer has padding 10.0.
    // Slot 0,0 is at 10.0, 10.0.

    if let Ok((mut z_index, mut node, original_pos, mut grid_pos, item_size)) = queries.get_mut(dropped_entity) {
        *z_index = ZIndex(1); // Reset Z-Index

        let slot_size = 50.0;
        let gap = 2.0;
        let padding = 10.0;

        let current_left = if let Val::Px(v) = node.left { v } else { 0.0 };
        let current_top = if let Val::Px(v) = node.top { v } else { 0.0 };

        // Calculate approximate grid index
        let col = ((current_left - padding + slot_size / 2.0) / (slot_size + gap)).floor() as i32;
        let row = ((current_top - padding + slot_size / 2.0) / (slot_size + gap)).floor() as i32;

        // Check bounds
        let max_col = 8 - item_size.width;
        let max_row = 8 - item_size.height;

        if col >= 0 && col <= max_col && row >= 0 && row <= max_row {
            // Valid placement (geometrically)
            // TODO: Check occupancy in InventoryGridState

            // Snap to grid
            let new_left = padding + (col as f32) * (slot_size + gap);
            let new_top = padding + (row as f32) * (slot_size + gap);

            node.left = Val::Px(new_left);
            node.top = Val::Px(new_top);

            grid_pos.x = col;
            grid_pos.y = row;

            println!("Snapped to {}, {}", col, row);
        } else {
            // Invalid placement, revert
            node.left = original_pos.0;
            node.top = original_pos.1;
            println!("Invalid placement, reverted.");
        }

        // Remove OriginalPosition
        commands.entity(dropped_entity).remove::<OriginalPosition>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_coord_calculation() {
        let padding = 10.0f32;
        let slot_size = 50.0f32;
        let gap = 2.0f32;

        // Test position for slot (0,0)
        let left = padding;
        let col = ((left - padding + slot_size / 2.0) / (slot_size + gap)).floor() as i32;
        assert_eq!(col, 0);

        // Test position for slot (1,0)
        let left_1 = padding + slot_size + gap;
        let col_1 = ((left_1 - padding + slot_size / 2.0) / (slot_size + gap)).floor() as i32;
        assert_eq!(col_1, 1);

        // Test position between slot 0 and 1 (closer to 1)
        let left_bias = padding + slot_size; // exactly on edge?
        // (50 + 25) / 52 = 1.44 -> 1
        let col_bias = ((left_bias - padding + slot_size / 2.0) / (slot_size + gap)).floor() as i32;
        assert_eq!(col_bias, 1);
    }

    #[test]
    fn test_boundary_check() {
        let grid_w = 8;
        let grid_h = 8;
        let item_w = 2; // 1x2 item
        let item_h = 1;

        // Valid position
        let x = 0;
        let y = 0;
        let max_col = grid_w - item_w; // 8 - 2 = 6
        let max_row = grid_h - item_h; // 8 - 1 = 7

        assert!(x >= 0 && x <= max_col);
        assert!(y >= 0 && y <= max_row);

        // Invalid position (too far right)
        let x_bad = 7;
        assert!(!(x_bad >= 0 && x_bad <= max_col));
    }
}
