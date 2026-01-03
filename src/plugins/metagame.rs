use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use crate::plugins::items::ItemDefinition;

// Re-export or redefine necessary types for serialization if they aren't in shared modules
// Since ItemDefinition is in items.rs, we import it.

#[derive(Resource, Debug, Serialize, Deserialize, Clone)]
pub struct SaveData {
    pub player_stats: PlayerStats,
    pub global_time: GlobalTime,
    pub inventory: Vec<SavedItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SavedItem {
    pub item_id: String,
    pub grid_x: i32,
    pub grid_y: i32,
    // We might want to save dynamic state here later (e.g. current durability, mutations)
}

#[derive(Resource, Debug, Serialize, Deserialize, Clone)]
pub struct PlayerStats {
    pub thalers: u32,
    pub reputation: u32,
    pub infection: u32,
}

impl Default for PlayerStats {
    fn default() -> Self {
        Self {
            thalers: 100,
            reputation: 50,
            infection: 0,
        }
    }
}

#[derive(Resource, Debug, Serialize, Deserialize, Clone)]
pub struct GlobalTime {
    pub day: u32,
    pub hour: u32, // 0-24
}

impl Default for GlobalTime {
    fn default() -> Self {
        Self {
            day: 1,
            hour: 6, // Starts at 6:00 AM
        }
    }
}

// Plugin
use crate::plugins::core::{GameState, DaySubState};
use crate::plugins::inventory::{InventoryGridState, GridPosition, Item, ItemSize, InventoryGridContainer, ItemSpawnedEvent};
use crate::plugins::items::ItemDatabase;
use std::fs::File;
use std::io::{Write, Read};

pub struct MetagamePlugin;

impl Plugin for MetagamePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerStats>()
           .init_resource::<GlobalTime>()
           .add_systems(OnEnter(DaySubState::Idle), day_start_logic)
           .add_systems(Update, (save_system, load_system_debug, debug_scene_transition)); // Add keyboard triggers for now
    }
}

fn debug_scene_transition(
    input: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
    current_state: Res<State<GameState>>,
) {
    if input.just_pressed(KeyCode::KeyT) {
        match current_state.get() {
            GameState::DayPhase => {
                info!("Transitioning to EveningPhase");
                next_state.set(GameState::EveningPhase);
            },
            GameState::EveningPhase => {
                info!("Transitioning to NightPhase");
                next_state.set(GameState::NightPhase);
            },
            GameState::NightPhase => {
                info!("Transitioning to DayPhase");
                next_state.set(GameState::DayPhase);
            },
            _ => {
                info!("Transitioning to DayPhase (default)");
                next_state.set(GameState::DayPhase);
            }
        }
    }
}

fn day_start_logic() {
    println!("Day Phase Started: Morning has broken.");
}

// Serialization Helpers

pub fn create_save_data(
    player_stats: &PlayerStats,
    global_time: &GlobalTime,
    q_items: &Query<(&ItemDefinition, &GridPosition), With<Item>>,
) -> SaveData {
    let mut saved_items = Vec::new();
    for (def, pos) in q_items.iter() {
        saved_items.push(SavedItem {
            item_id: def.id.clone(),
            grid_x: pos.x,
            grid_y: pos.y,
        });
    }

    SaveData {
        player_stats: player_stats.clone(),
        global_time: global_time.clone(),
        inventory: saved_items,
    }
}

fn save_system(
    input: Res<ButtonInput<KeyCode>>,
    player_stats: Res<PlayerStats>,
    global_time: Res<GlobalTime>,
    q_items: Query<(&ItemDefinition, &GridPosition), With<Item>>,
) {
    if input.just_pressed(KeyCode::F5) {
        let save_data = create_save_data(&player_stats, &global_time, &q_items);

        match serde_json::to_string_pretty(&save_data) {
            Ok(json) => {
                if let Ok(mut file) = File::create("savegame.json") {
                    if let Err(e) = file.write_all(json.as_bytes()) {
                        error!("Failed to write save file: {}", e);
                    } else {
                        info!("Game saved successfully to savegame.json");
                    }
                } else {
                    error!("Failed to create save file");
                }
            },
            Err(e) => error!("Failed to serialize save data: {}", e),
        }
    }
}

fn load_system_debug(
    input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut player_stats: ResMut<PlayerStats>,
    mut global_time: ResMut<GlobalTime>,
    mut grid_state: ResMut<InventoryGridState>,
    item_db: Res<ItemDatabase>,
    q_items: Query<Entity, With<Item>>,
    q_container: Query<Entity, With<InventoryGridContainer>>,
) {
    if input.just_pressed(KeyCode::F9) {
        if let Ok(mut file) = File::open("savegame.json") {
            let mut json = String::new();
            if file.read_to_string(&mut json).is_ok() {
                match serde_json::from_str::<SaveData>(&json) {
                    Ok(data) => {
                        // Apply loaded state
                        *player_stats = data.player_stats;
                        *global_time = data.global_time;

                        // Clear current inventory
                        for entity in q_items.iter() {
                            commands.entity(entity).despawn_recursive();
                        }
                        grid_state.cells.clear();

                        // Respawn items
                        if let Ok(container) = q_container.get_single() {
                            for saved_item in data.inventory {
                                if let Some(def) = item_db.items.get(&saved_item.item_id) {
                                     let size = ItemSize { width: def.width as i32, height: def.height as i32 };
                                     let pos = IVec2::new(saved_item.grid_x, saved_item.grid_y);

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
                                            display: Display::Flex,
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
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
                                            Text::new(def.name.clone()),
                                            TextFont {
                                                font_size: 12.0,
                                                ..default()
                                            },
                                            TextColor(Color::WHITE),
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
                                }
                            }
                        }

                        info!("Game loaded successfully.");
                    },
                    Err(e) => error!("Failed to deserialize save data: {}", e),
                }
            }
        } else {
            warn!("No save file found.");
        }
    }
}
