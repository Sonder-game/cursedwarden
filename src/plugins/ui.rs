use bevy::prelude::*;
use crate::plugins::metagame::{PlayerStats, GlobalTime};
use crate::plugins::core::GameState;
use crate::plugins::items::ItemDefinition;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_hud)
           .add_systems(Update, (update_hud, tooltip_system));
    }
}

// Marker components for UI updates
#[derive(Component)]
struct PhaseText;

#[derive(Component)]
struct StatsText;

// Tooltip Marker
#[derive(Component)]
struct TooltipNode;

#[derive(Component)]
struct TooltipText;

fn spawn_hud(mut commands: Commands) {
    // Root UI Node (Overlay)
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            justify_content: JustifyContent::SpaceBetween,
            flex_direction: FlexDirection::Column,
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
                Text::new("Controls: [Space] Spawn Item (Eve) | [T] Next Phase | [F5] Save | [F9] Load | [Drag] Move Items | [Alt] Tooltips"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
            ));

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

    // Spawn hidden Tooltip
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            padding: UiRect::all(Val::Px(10.0)),
            border: UiRect::all(Val::Px(2.0)),
            display: Display::None, // Hidden by default
            flex_direction: FlexDirection::Column,
            max_width: Val::Px(300.0),
            ..default()
        },
        BackgroundColor(Color::srgb(0.1, 0.1, 0.1).with_alpha(0.95)),
        BorderColor(Color::WHITE),
        TooltipNode,
        ZIndex(300), // Topmost
        PickingBehavior::IGNORE,
    )).with_children(|parent| {
        parent.spawn((
            Text::new("Tooltip"),
            TextFont { font_size: 14.0, ..default() },
            TextColor(Color::WHITE),
            TooltipText
        ));
    });
}

#[derive(Component)]
struct StartCombatButton;

fn update_hud(
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

fn tooltip_system(
    mut q_tooltip: Query<(&mut Node, &mut Display), With<TooltipNode>>,
    mut q_text: Query<&mut Text, With<TooltipText>>,
    q_interacted: Query<(&Interaction, &ItemDefinition, &GlobalTransform), With<crate::plugins::inventory::Item>>,
    input: Res<ButtonInput<KeyCode>>,
    q_window: Query<&Window>,
) {
    let show_tooltip = input.pressed(KeyCode::AltLeft) || input.pressed(KeyCode::AltRight);

    if let Ok((mut node, mut display)) = q_tooltip.get_single_mut() {
        if !show_tooltip {
             *display = Display::None;
             return;
        }

        // Find hovered item
        let mut found = false;
        if let Ok(window) = q_window.get_single() {
             if let Some(cursor_pos) = window.cursor_position() {
                 // Simple hover check from interaction
                 for (interaction, def, transform) in q_interacted.iter() {
                     if *interaction == Interaction::Hovered {
                          found = true;
                          *display = Display::Flex;

                          // Position tooltip near cursor
                          node.left = Val::Px(cursor_pos.x + 15.0);
                          node.top = Val::Px(cursor_pos.y + 15.0);

                          if let Ok(mut text) = q_text.get_single_mut() {
                              let mut content = format!("{}\n\n{}", def.name, def.description);
                              if def.attack > 0.0 { content.push_str(&format!("\nAttack: {}", def.attack)); }
                              if def.defense > 0.0 { content.push_str(&format!("\nDefense: {}", def.defense)); }
                              if def.speed != 0.0 { content.push_str(&format!("\nSpeed: {}", def.speed)); }
                              content.push_str(&format!("\nRarity: {:?}\nPrice: {}", def.rarity, def.price));

                              **text = content;
                          }
                          break;
                     }
                 }
             }
        }

        if !found {
            *display = Display::None;
        }
    }
}
