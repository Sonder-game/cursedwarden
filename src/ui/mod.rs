use bevy::prelude::*;
use crate::core::GameState;
use crate::inventory::{InventorySlot, InventoryGridState};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::EveningPhase), spawn_inventory_ui);
    }
}

fn spawn_inventory_ui(mut commands: Commands, mut grid_state: ResMut<InventoryGridState>) {
    // Initialize grid state for demo purposes if it's empty
    if grid_state.width == 0 {
        grid_state.width = 10;
        grid_state.height = 10;
    }

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
            // Inventory Container
            parent
                .spawn(Node {
                    display: Display::Grid,
                    grid_template_columns: vec![GridTrack::px(50.0); grid_state.width as usize],
                    grid_template_rows: vec![GridTrack::px(50.0); grid_state.height as usize],
                    column_gap: Val::Px(2.0),
                    row_gap: Val::Px(2.0),
                    padding: UiRect::all(Val::Px(10.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                })
                .with_children(|grid| {
                    for y in 0..grid_state.height {
                        for x in 0..grid_state.width {
                            grid.spawn((
                                Node {
                                    width: Val::Px(50.0),
                                    height: Val::Px(50.0),
                                    border: UiRect::all(Val::Px(1.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.2, 0.2, 0.2).into()),
                                BorderColor(Color::srgb(0.5, 0.5, 0.5)),
                                InventorySlot {
                                    position: IVec2::new(x, y),
                                },
                            ));
                        }
                    }
                });
        });

    println!("Inventory UI spawned with size {}x{}", grid_state.width, grid_state.height);
}
