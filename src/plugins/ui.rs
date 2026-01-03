use bevy::prelude::*;
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
        // PickingBehavior::Ignore,
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
        });
    });
}

fn update_hud(
    state: Res<State<GameState>>,
    player_stats: Res<PlayerStats>,
    time: Res<GlobalTime>,
    mut q_phase: Query<&mut Text, (With<PhaseText>, Without<StatsText>)>,
    mut q_stats: Query<&mut Text, (With<StatsText>, Without<PhaseText>)>,
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
}
