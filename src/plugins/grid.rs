use bevy::prelude::*;
use bevy::picking::prelude::*;
use std::collections::HashMap;
use crate::plugins::core::GameState;

pub struct GridInventoryPlugin;

impl Plugin for GridInventoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InventoryGridState>()
            //.add_plugins(DefaultPickingPlugins) // Usually included in DefaultPlugins, check if needed
            .add_systems(OnEnter(GameState::EveningPhase), spawn_inventory_ui)
            //.add_systems(Update, drag_end_system.run_if(in_state(GameState::EveningPhase)))
            ;
    }
}

// Components

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridPosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct ItemSize {
    pub width: i32,
    pub height: i32,
}

#[derive(Component)]
pub struct Item;

#[derive(Component)]
pub struct InventorySlot {
    pub x: i32,
    pub y: i32,
}

#[derive(Component)]
pub struct DragOriginalState {
    pub original_left: Val,
    pub original_top: Val,
    pub original_grid_pos: GridPosition,
}

// Constants
const TILE_SIZE: f32 = 50.0;
const GRID_GAP: f32 = 2.0;
const GRID_PADDING: f32 = 10.0;
const GRID_WIDTH: i32 = 10;
const GRID_HEIGHT: i32 = 8;
const FULL_TILE_SIZE: f32 = TILE_SIZE + GRID_GAP; // 52.0

// Resource

#[derive(Resource, Default)]
pub struct InventoryGridState {
    pub cells: HashMap<IVec2, Entity>,
    pub width: i32,
    pub height: i32,
}

// Systems

fn spawn_inventory_ui(
    mut commands: Commands,
    mut grid_state: ResMut<InventoryGridState>
) {
    // Setup grid dimensions
    grid_state.width = GRID_WIDTH;
    grid_state.height = GRID_HEIGHT;

    commands.spawn(Node {
        display: Display::Grid,
        grid_template_columns: vec![GridTrack::px(TILE_SIZE); GRID_WIDTH as usize],
        grid_template_rows: vec![GridTrack::px(TILE_SIZE); GRID_HEIGHT as usize],
        row_gap: Val::Px(GRID_GAP),
        column_gap: Val::Px(GRID_GAP),
        padding: UiRect::all(Val::Px(GRID_PADDING)),
        ..default()
    }).with_children(|parent| {
        for y in 0..GRID_HEIGHT {
            for x in 0..GRID_WIDTH {
                parent.spawn((
                    Node {
                        width: Val::Px(TILE_SIZE),
                        height: Val::Px(TILE_SIZE),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.2, 0.2).into()),
                    BorderColor::from(Color::BLACK),
                    InventorySlot { x, y },
                    // Pickable is not needed in Bevy 0.15 if we use observers or it might be default?
                    // Or it's in bevy::picking::prelude::Pickable.
                    // But wait, the previous code had use bevy::picking::prelude::*; removed.
                    // Let's check imports.
                ));
            }
        }
    });

    // Spawn a test item
    let item_entity = spawn_item(&mut commands, 0, 0, 2, 2, Color::srgb(0.8, 0.2, 0.2));

    // Manually register initial item in grid state
    for i in 0..2 {
        for j in 0..2 {
            grid_state.cells.insert(IVec2::new(0 + i, 0 + j), item_entity);
        }
    }
}

fn spawn_item(commands: &mut Commands, x: i32, y: i32, w: i32, h: i32, color: Color) -> Entity {
    commands.spawn((
        Node {
            width: Val::Px(TILE_SIZE * w as f32),
            height: Val::Px(TILE_SIZE * h as f32),
            position_type: PositionType::Absolute,
            left: Val::Px(x as f32 * FULL_TILE_SIZE + GRID_PADDING),
            top: Val::Px(y as f32 * FULL_TILE_SIZE + GRID_PADDING),
            ..default()
        },
        BackgroundColor(color.into()),
        Item,
        GridPosition { x, y },
        ItemSize { width: w, height: h },
        // Pickable::default(),
    ))
    .observe(drag_start)
    .observe(drag_system)
    .observe(drag_end)
    .id()
}

fn drag_start(
    trigger: Trigger<Pointer<DragStart>>,
    mut commands: Commands,
    mut query: Query<(&mut ZIndex, &Node, &GridPosition)>
) {
     // Bring to front and store original state
     if let Ok((mut z_index, node, grid_pos)) = query.get_mut(trigger.entity()) {
         *z_index = ZIndex(100);
         commands.entity(trigger.entity()).insert(DragOriginalState {
             original_left: node.left,
             original_top: node.top,
             original_grid_pos: *grid_pos,
         });
     }
}

fn drag_system(
    trigger: Trigger<Pointer<Drag>>,
    mut transforms: Query<&mut Node>,
) {
    let event = trigger.event();
    if let Ok(mut node) = transforms.get_mut(trigger.entity()) {
        if let Val::Px(current_left) = node.left {
            node.left = Val::Px(current_left + event.delta.x);
        }
        if let Val::Px(current_top) = node.top {
            node.top = Val::Px(current_top + event.delta.y);
        }
    }
}

fn drag_end(
    trigger: Trigger<Pointer<DragEnd>>,
    mut commands: Commands,
    mut query: Query<(&mut GridPosition, &mut Node, &mut ZIndex, &ItemSize, &DragOriginalState)>,
    mut grid_state: ResMut<InventoryGridState>,
) {
    let entity = trigger.entity();

    // Reset ZIndex
    if let Ok((mut grid_pos, mut node, mut z_index, item_size, original_state)) = query.get_mut(entity) {
        *z_index = ZIndex(0);

        // Snap logic
        let mut valid_move = false;
        let mut new_x = 0;
        let mut new_y = 0;

        if let (Val::Px(x_px), Val::Px(y_px)) = (node.left, node.top) {
            new_x = ((x_px - GRID_PADDING) / FULL_TILE_SIZE).round() as i32;
            new_y = ((y_px - GRID_PADDING) / FULL_TILE_SIZE).round() as i32;

            // 1. Bounds check
            // Check if all cells of the item are within bounds
            if new_x >= 0 && new_y >= 0 &&
               new_x + item_size.width <= grid_state.width &&
               new_y + item_size.height <= grid_state.height
            {
                // 2. Collision check
                let mut collision = false;
                for i in 0..item_size.width {
                    for j in 0..item_size.height {
                        let check_pos = IVec2::new(new_x + i, new_y + j);
                        if let Some(&occupier) = grid_state.cells.get(&check_pos) {
                            if occupier != entity {
                                collision = true;
                                break;
                            }
                        }
                    }
                    if collision { break; }
                }

                if !collision {
                    valid_move = true;
                }
            }
        }

        if valid_move {
            // Remove old cells from grid state
            for i in 0..item_size.width {
                for j in 0..item_size.height {
                    let old_pos = IVec2::new(original_state.original_grid_pos.x + i, original_state.original_grid_pos.y + j);
                    if let Some(owner) = grid_state.cells.get(&old_pos) {
                        if *owner == entity {
                            grid_state.cells.remove(&old_pos);
                        }
                    }
                }
            }

            // Update GridPosition
            grid_pos.x = new_x;
            grid_pos.y = new_y;

            // Add new cells to grid state
            for i in 0..item_size.width {
                for j in 0..item_size.height {
                    let new_pos = IVec2::new(new_x + i, new_y + j);
                    grid_state.cells.insert(new_pos, entity);
                }
            }

            // Snap visual position
            node.left = Val::Px(new_x as f32 * FULL_TILE_SIZE + GRID_PADDING);
            node.top = Val::Px(new_y as f32 * FULL_TILE_SIZE + GRID_PADDING);
        } else {
            // Revert
            node.left = original_state.original_left;
            node.top = original_state.original_top;
            *grid_pos = original_state.original_grid_pos;
        }

        // Clean up
        commands.entity(entity).remove::<DragOriginalState>();
    }
}
