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
    #[serde(default)]
    pub rotation: u8,
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
use crate::plugins::inventory::{InventoryGridState, GridPosition, Item, ItemSize, InventoryGridContainer, ItemSpawnedEvent, CellState, ItemRotation};
use crate::plugins::items::ItemDatabase;
use std::fs::File;
use std::io::{Write, Read};

pub struct MetagamePlugin;

#[derive(Resource, Default, Debug)]
pub struct PendingItems(pub Vec<String>);

/// Holds inventory state between Evening phases (e.g. during Combat)
#[derive(Resource, Debug, Clone)]
pub struct PersistentInventory {
    pub items: Vec<SavedItem>,
}

impl Default for PersistentInventory {
    fn default() -> Self {
        Self {
            items: vec![
                // Starter Bag at center-ish
                SavedItem {
                    item_id: "starter_bag".to_string(),
                    grid_x: 2,
                    grid_y: 2,
                    rotation: 0,
                }
            ],
        }
    }
}

#[derive(Component)]
struct CityUiRoot;

#[derive(Component)]
struct CityButton(pub &'static str);

impl Plugin for MetagamePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerStats>()
           .init_resource::<GlobalTime>()
           .init_resource::<PendingItems>()
           .init_resource::<PersistentInventory>()
           .add_systems(OnEnter(DaySubState::Idle), day_start_logic)
           .add_systems(OnEnter(GameState::DayPhase), spawn_city_ui)
           .add_systems(OnExit(GameState::DayPhase), cleanup_city_ui)
           .add_systems(Update, handle_city_buttons.run_if(in_state(GameState::DayPhase)))
           .add_systems(Update, (save_system, load_system_debug, debug_scene_transition)); // Add keyboard triggers for now
    }
}

fn spawn_city_ui(mut commands: Commands) {
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            display: Display::Flex,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(20.0),
            ..default()
        },
        BackgroundColor(Color::srgb(0.1, 0.1, 0.15)),
        CityUiRoot,
    ))
    .with_children(|parent| {
        parent.spawn((
            Text::new("City Phase\nExplore locations to find items"),
            TextFont { font_size: 30.0, ..default() },
            TextColor(Color::WHITE),
            Node { margin: UiRect::bottom(Val::Px(20.0)), ..default() },
        ));

        let buttons = [
            ("Visit Market (Sword)", "steel_sword"),
            ("Visit Slums (Dagger)", "silver_dagger"),
            ("Go to Inventory", "NEXT_PHASE"),
        ];

        for (label, action) in buttons {
            parent.spawn((
                Button,
                Node {
                    width: Val::Px(200.0),
                    height: Val::Px(50.0),
                    border: UiRect::all(Val::Px(2.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BorderColor(Color::BLACK),
                BackgroundColor(Color::srgb(0.3, 0.3, 0.4)),
                CityButton(action),
            ))
            .with_children(|p| {
                p.spawn((
                    Text::new(label),
                    TextFont { font_size: 20.0, ..default() },
                    TextColor(Color::WHITE),
                ));
            });
        }
    });
}

fn cleanup_city_ui(mut commands: Commands, q_root: Query<Entity, With<CityUiRoot>>) {
    for e in q_root.iter() {
        commands.entity(e).despawn_recursive();
    }
}

fn handle_city_buttons(
    // Removed unused mut commands
    mut q_buttons: Query<(&Interaction, &CityButton, &mut BackgroundColor), (Changed<Interaction>, With<Button>)>,
    mut pending_items: ResMut<PendingItems>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for (interaction, action, mut bg_color) in q_buttons.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *bg_color = BackgroundColor(Color::srgb(0.2, 0.2, 0.3));
                if action.0 == "NEXT_PHASE" {
                    next_state.set(GameState::EveningPhase);
                } else {
                    pending_items.0.push(action.0.to_string());
                    info!("Found item: {}", action.0);
                }
            },
            Interaction::Hovered => {
                *bg_color = BackgroundColor(Color::srgb(0.4, 0.4, 0.5));
            },
            Interaction::None => {
                *bg_color = BackgroundColor(Color::srgb(0.3, 0.3, 0.4));
            },
        }
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
    q_items: &Query<(&ItemDefinition, &GridPosition, &ItemRotation), With<Item>>,
) -> SaveData {
    let mut saved_items = Vec::new();
    for (def, pos, rot) in q_items.iter() {
        saved_items.push(SavedItem {
            item_id: def.id.clone(),
            grid_x: pos.x,
            grid_y: pos.y,
            rotation: rot.value,
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
    q_items: Query<(&ItemDefinition, &GridPosition, &ItemRotation), With<Item>>,
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
                        // grid_state.cells.clear();
                        for cell in grid_state.grid.values_mut() {
                            cell.state = CellState::Free;
                        }

                        // Respawn items
                        if let Ok(container) = q_container.get_single() {
                            for saved_item in data.inventory {
                                if let Some(def) = item_db.items.get(&saved_item.item_id) {
                                     let rotation = saved_item.rotation;
                                     let rotated_shape = InventoryGridState::get_rotated_shape(&def.shape, rotation);

                                     // Recalculate size from shape
                                     let mut min_x = 0;
                                     let mut max_x = 0;
                                     let mut min_y = 0;
                                     let mut max_y = 0;
                                     if !rotated_shape.is_empty() {
                                         min_x = rotated_shape[0].x;
                                         max_x = rotated_shape[0].x;
                                         min_y = rotated_shape[0].y;
                                         max_y = rotated_shape[0].y;
                                         for p in &rotated_shape {
                                             if p.x < min_x { min_x = p.x; }
                                             if p.x > max_x { max_x = p.x; }
                                             if p.y < min_y { min_y = p.y; }
                                             if p.y > max_y { max_y = p.y; }
                                         }
                                     }
                                     let width_slots = max_x - min_x + 1;
                                     let height_slots = max_y - min_y + 1;

                                     let pos = IVec2::new(saved_item.grid_x, saved_item.grid_y);

                                     // Visuals
                                     let effective_x = pos.x + min_x;
                                     let effective_y = pos.y + min_y;

                                     let left = 10.0 + effective_x as f32 * 52.0;
                                     let top = 10.0 + effective_y as f32 * 52.0;
                                     let width = width_slots as f32 * 50.0 + (width_slots - 1) as f32 * 2.0;
                                     let height = height_slots as f32 * 50.0 + (height_slots - 1) as f32 * 2.0;

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
                                        Interaction::default(),
                                        Item,
                                        GridPosition { x: pos.x, y: pos.y },
                                        ItemSize { width: width_slots, height: height_slots },
                                        ItemRotation { value: rotation },
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
                                    for offset in rotated_shape {
                                        let cell_pos = pos + offset;
                                        if let Some(cell) = grid_state.grid.get_mut(&cell_pos) {
                                            cell.state = CellState::Occupied(item_entity);
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
