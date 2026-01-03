use bevy::prelude::*;
use bevy::utils::HashMap;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};

use crate::plugins::core::GameState;
use crate::plugins::items::{ItemDefinition, ItemDatabase};
use crate::plugins::inventory::{InventoryGridState, GridPosition, ItemSize, Item, InventoryGridContainer};

pub struct MetagamePlugin;

impl Plugin for MetagamePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerStats>()
            .init_resource::<GlobalTime>()
            .add_systems(Update, (game_time_system, save_load_input_system))
            .add_systems(OnEnter(GameState::DayPhase), on_enter_day_phase);
    }
}

#[derive(Resource, Default, Serialize, Deserialize, Debug, Clone)]
pub struct PlayerStats {
    pub thalers: u32,
    pub reputation: u32,
    pub infection: u32,
}

#[derive(Resource, Default, Serialize, Deserialize, Debug, Clone)]
pub struct GlobalTime {
    pub day: u32,
    pub hour: u32, // 0-23
}

#[derive(Serialize, Deserialize)]
pub struct GridItemSaveData {
    pub item_id: String,
    pub grid_position: GridPosition,
    pub size: ItemSize,
}

#[derive(Serialize, Deserialize)]
pub struct SaveData {
    pub player_stats: PlayerStats,
    pub global_time: GlobalTime,
    pub inventory_items: Vec<GridItemSaveData>,
}

fn game_time_system(_time: ResMut<GlobalTime>) {
    // Placeholder for time progression logic
}

fn on_enter_day_phase() {
    info!("Entered Day Phase");
    // Logic for day phase start
}

fn save_load_input_system(
    input: Res<ButtonInput<KeyCode>>,
    player_stats: Res<PlayerStats>,
    global_time: Res<GlobalTime>,
    q_items: Query<(&ItemDefinition, &GridPosition, &ItemSize), With<Item>>,
    mut commands: Commands,
    mut grid_state: ResMut<InventoryGridState>,
    item_db: Res<ItemDatabase>,
    q_container: Query<Entity, With<InventoryGridContainer>>,
    q_existing_items: Query<Entity, With<Item>>,
) {
    if input.just_pressed(KeyCode::F5) {
        // SAVE
        let mut inventory_items = Vec::new();
        for (def, pos, size) in q_items.iter() {
            inventory_items.push(GridItemSaveData {
                item_id: def.id.clone(),
                grid_position: *pos,
                size: *size,
            });
        }

        let save_data = SaveData {
            player_stats: player_stats.clone(),
            global_time: global_time.clone(),
            inventory_items,
        };

        if let Ok(serialized) = ron::ser::to_string(&save_data) {
             if let Ok(mut file) = File::create("savegame.ron") {
                 if let Err(e) = file.write_all(serialized.as_bytes()) {
                     error!("Failed to write save file: {}", e);
                 } else {
                     info!("Game saved to savegame.ron");
                 }
             }
        }
    }

    if input.just_pressed(KeyCode::F9) {
        // LOAD
        if let Ok(mut file) = File::open("savegame.ron") {
            let mut contents = String::new();
            if file.read_to_string(&mut contents).is_ok() {
                if let Ok(save_data) = ron::from_str::<SaveData>(&contents) {
                    info!("Loading game...");

                    // 1. Restore Resources
                    commands.insert_resource(save_data.player_stats);
                    commands.insert_resource(save_data.global_time);

                    // 2. Clear Inventory
                    for entity in q_existing_items.iter() {
                        commands.entity(entity).despawn_recursive();
                    }
                    grid_state.cells.clear();

                    // 3. Respawn Items
                    if let Ok(container) = q_container.get_single() {
                         for item_data in save_data.inventory_items {
                             // Check if definition exists (robustness)
                             let mut def = item_data.item_id.clone();
                             let mut item_def = ItemDefinition {
                                 id: "unknown".to_string(),
                                 name: "Unknown".to_string(),
                                 width: item_data.size.width as u8,
                                 height: item_data.size.height as u8,
                                 material: crate::plugins::items::MaterialType::Steel,
                                 item_type: crate::plugins::items::ItemType::Consumable,
                             };

                             if let Some(db_def) = item_db.items.get(&item_data.item_id) {
                                 item_def = db_def.clone();
                             } else {
                                 warn!("Item definition not found for ID: {}, using placeholder", item_data.item_id);
                             }

                             let pos = item_data.grid_position;
                             let size = item_data.size;

                             // Calculate UI position
                             let left = 10.0 + pos.x as f32 * 52.0;
                             let top = 10.0 + pos.y as f32 * 52.0;
                             let width = size.width as f32 * 50.0 + (size.width - 1) as f32 * 2.0;
                             let height = size.height as f32 * 50.0 + (size.height - 1) as f32 * 2.0;

                             // Spawn the item using the same logic as debug_spawn_item_system
                             // Note: We need to import the handlers from inventory module
                             use crate::plugins::inventory::{handle_drag, handle_drag_drop, handle_drag_start};

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
                                item_def, // Attach the definition
                            ))
                            .observe(handle_drag_start)
                            .observe(handle_drag)
                            .observe(handle_drag_drop)
                            .id();

                            // Add to grid state
                            for dy in 0..size.height {
                                for dx in 0..size.width {
                                    grid_state.cells.insert(IVec2::new(pos.x + dx, pos.y + dy), item_entity);
                                }
                            }

                            // Attach to container
                            commands.entity(container).add_child(item_entity);

                            info!("Restored item {} at {:?}", item_data.item_id, pos);
                         }
                    }
                }
            }
        }
    }
}
