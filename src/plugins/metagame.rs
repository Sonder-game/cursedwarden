use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Write, Read};
use crate::plugins::inventory::{InventoryGridState, Item, GridPosition, ItemSize};
use crate::plugins::items::ItemDefinition;
use crate::plugins::core::GameState;

pub struct MetagamePlugin;

impl Plugin for MetagamePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerStats>()
            .init_resource::<GlobalTime>()
            .add_systems(Update, (
                game_time_system,
                save_game_input_system,
                load_game_input_system
            ))
            .add_systems(OnEnter(GameState::DayPhase), day_phase_start);
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
pub struct SavedItem {
    pub id: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Serialize, Deserialize)]
pub struct SaveData {
    pub player_stats: PlayerStats,
    pub time: GlobalTime,
    pub inventory_items: Vec<SavedItem>,
}

fn game_time_system(_time: ResMut<GlobalTime>) {
    // Placeholder for time progression logic
}

fn day_phase_start() {
    info!("Entering Day Phase: Time management and interactions enabled.");
}

fn save_game_input_system(
    input: Res<ButtonInput<KeyCode>>,
    player_stats: Res<PlayerStats>,
    time: Res<GlobalTime>,
    q_items: Query<(&GridPosition, &ItemSize, &ItemDefinition), With<Item>>,
) {
    if input.just_pressed(KeyCode::F5) {
        info!("Saving game...");

        let mut inventory_items = Vec::new();
        for (pos, size, def) in q_items.iter() {
            inventory_items.push(SavedItem {
                id: def.id.clone(),
                x: pos.x,
                y: pos.y,
                width: size.width,
                height: size.height,
            });
        }

        let save_data = SaveData {
            player_stats: player_stats.clone(),
            time: time.clone(),
            inventory_items,
        };

        if let Ok(serialized) = ron::ser::to_string(&save_data) {
            // Ensure directory exists
            if let Err(e) = std::fs::create_dir_all("saves") {
                error!("Failed to create saves directory: {}", e);
                return;
            }

            if let Ok(mut file) = File::create("saves/savegame.ron") {
                if let Err(e) = file.write_all(serialized.as_bytes()) {
                     error!("Failed to write to save file: {}", e);
                } else {
                     info!("Game saved successfully to saves/savegame.ron");
                }
            } else {
                error!("Failed to create save file");
            }
        }
    }
}

fn load_game_input_system(
    input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut player_stats: ResMut<PlayerStats>,
    mut time: ResMut<GlobalTime>,
    mut grid_state: ResMut<InventoryGridState>,
    q_items: Query<Entity, With<Item>>,
    q_container: Query<Entity, With<crate::plugins::inventory::InventoryGridContainer>>,
    item_db: Res<crate::plugins::items::ItemDatabase>,
) {
    if input.just_pressed(KeyCode::F9) {
        info!("Loading game...");

        if let Ok(mut file) = File::open("saves/savegame.ron") {
            let mut contents = String::new();
            if file.read_to_string(&mut contents).is_ok() {
                if let Ok(save_data) = ron::de::from_str::<SaveData>(&contents) {
                    // Restore stats
                    *player_stats = save_data.player_stats;
                    *time = save_data.time;

                    // Clear existing items
                    for entity in q_items.iter() {
                        commands.entity(entity).despawn_recursive();
                    }
                    grid_state.cells.clear();

                    // Restore items
                    if let Ok(container) = q_container.get_single() {
                         for saved_item in save_data.inventory_items {
                            if let Some(def) = item_db.items.get(&saved_item.id) {
                                let size = ItemSize { width: saved_item.width, height: saved_item.height };
                                let pos = IVec2::new(saved_item.x, saved_item.y);

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
                                .observe(crate::plugins::inventory::handle_drag_start)
                                .observe(crate::plugins::inventory::handle_drag)
                                .observe(crate::plugins::inventory::handle_drag_drop)
                                .id();

                                // Update grid state
                                for dy in 0..size.height {
                                    for dx in 0..size.width {
                                        grid_state.cells.insert(IVec2::new(pos.x + dx, pos.y + dy), item_entity);
                                    }
                                }

                                commands.entity(container).add_child(item_entity);
                            }
                         }
                    }

                    info!("Game loaded successfully.");
                } else {
                    error!("Failed to deserialize save data");
                }
            } else {
                error!("Failed to read save file");
            }
        } else {
            error!("Save file not found");
        }
    }
}
