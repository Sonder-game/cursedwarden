use bevy::prelude::*;
use bevy::state::prelude::*;
use crate::plugins::metagame::{PlayerStats, GlobalTime};
use crate::plugins::core::GameState;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_hud)
           .add_systems(Update, update_hud);
    }
}

// Marker components for UI updates
#[derive(Component)]
struct PhaseText;

#[derive(Component)]
struct StatsText;

fn spawn_hud(mut commands: Commands) {
    // Root UI Node (Overlay)
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            justify_content: JustifyContent::SpaceBetween,
            flex_direction: FlexDirection::Column,
            // pointer_events removed, using PickingBehavior if needed or default.
            // Bevy 0.15 defaults to passing through if no interaction components?
            // Actually, Node blocks clicks by default in picking.
            // We need PickingBehavior::Ignore.
            ..default()
        },
        PickingBehavior::IGNORE,
        ZIndex(200), // Above everything
    ))
    .with_children(|parent| {
        // Top Bar (Stats & Info)
        parent.spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(40.0),
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(Val::Px(10.0)),
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            BackgroundColor(Color::srgb(0.0, 0.0, 0.0).with_alpha(0.8)),
        ))
        .with_children(|top_bar| {
            // Phase Display
            top_bar.spawn((
                Text::new("Phase: Init"),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                PhaseText,
            ));

            // Stats Display
            top_bar.spawn((
                Text::new("Thalers: 0 | Rep: 0 | Inf: 0"),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.8, 0.2)), // Gold-ish
                StatsText,
            ));
        });

        // Bottom Bar (Controls)
        parent.spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(30.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.0, 0.0, 0.0).with_alpha(0.6)),
        ))
        .with_children(|bottom_bar| {
             bottom_bar.spawn((
                Text::new("Controls: [Space] Spawn Item (Eve) | [T] Next Phase | [F5] Save | [F9] Load | [Drag] Move Items"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
            ));

             // Start Combat Button (Visible in Evening Only - Logic below needs to handle visibility, or we spawn it dynamically elsewhere.
             // For simplicity, let's spawn it here but toggle visibility in update_hud, or just add a button that is always there but only works in Evening?
             // Better: Add a distinct UI element for the button.)
             bottom_bar.spawn((
                 Button,
                 Node {
                     width: Val::Px(120.0),
                     height: Val::Px(24.0),
                     margin: UiRect::left(Val::Px(20.0)),
                     justify_content: JustifyContent::Center,
                     align_items: AlignItems::Center,
                     ..default()
                 },
                 BackgroundColor(Color::srgb(0.6, 0.1, 0.1)),
                 StartCombatButton,
             ))
             .with_children(|btn| {
                 btn.spawn((
                     Text::new("Start Combat"),
                     TextFont { font_size: 14.0, ..default() },
                     TextColor(Color::WHITE),
                 ));
             });
        });
    });
}

#[derive(Component)]
struct StartCombatButton;

fn update_hud(
    // Removed unused mut commands
    state: Res<State<GameState>>,
    player_stats: Res<PlayerStats>,
    time: Res<GlobalTime>,
    mut q_phase: Query<&mut Text, (With<PhaseText>, Without<StatsText>)>,
    mut q_stats: Query<&mut Text, (With<StatsText>, Without<PhaseText>)>,
    mut q_combat_btn: Query<&mut Visibility, With<StartCombatButton>>,
    q_interaction: Query<&Interaction, (Changed<Interaction>, With<StartCombatButton>)>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    // Update Phase Text
    for mut text in q_phase.iter_mut() {
        let phase_name = match state.get() {
            GameState::AssetLoading => "Loading...",
            GameState::MainMenu => "Main Menu",
            GameState::DayPhase => "Day Phase (Metagame)",
            GameState::EveningPhase => "Evening Phase (Inventory)",
            GameState::NightPhase => "Night Phase (Combat)",
            GameState::EventResolution => "Event",
            GameState::GameOver => "Game Over",
        };
        let time_str = format!("Day {} {:02}:00", time.day, time.hour);
        **text = format!("{} - {}", phase_name, time_str);
    }

    // Update Stats Text
    for mut text in q_stats.iter_mut() {
        **text = format!(
            "Thalers: {} | Rep: {} | Inf: {}",
            player_stats.thalers, player_stats.reputation, player_stats.infection
        );
    }

    // Handle Combat Button Visibility & Click
    let show_button = *state.get() == GameState::EveningPhase;
    for mut vis in q_combat_btn.iter_mut() {
        *vis = if show_button { Visibility::Visible } else { Visibility::Hidden };
    }

    if show_button {
        for interaction in q_interaction.iter() {
            if *interaction == Interaction::Pressed {
                info!("Starting Combat!");
                next_state.set(GameState::NightPhase);
            }
        }
    }
}
