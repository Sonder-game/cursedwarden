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

// Marker components
#[derive(Component)] struct PhaseText;
#[derive(Component)] struct StatsText;
#[derive(Component)] struct StartCombatButton;

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
       // CRITICAL: Ignore picking on root transparent container,
       // so clicks pass through to inventory
       PickingBehavior::IGNORE,
       ZIndex(200), // On top of everything
   ))
  .with_children(|parent| {
       // Top Bar
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
           // Here PickingBehavior is default (BLOCK), so buttons work
       ))
      .with_children(|top_bar| {
           top_bar.spawn((
               Text::new("Phase: Init"),
               TextFont { font_size: 20.0,..default() },
               TextColor(Color::WHITE),
               PhaseText,
           ));
           top_bar.spawn((
               Text::new("Stats..."),
               TextFont { font_size: 20.0,..default() },
               TextColor(Color::srgb(1.0, 0.84, 0.0)),
               StatsText,
           ));
       });

       // Bottom Bar
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
                Button,
                Node {
                    width: Val::Px(120.0),
                    height: Val::Px(24.0),
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
                    TextFont { font_size: 14.0,..default() },
                    TextColor(Color::WHITE),
                    PickingBehavior::IGNORE,
                ));
            });
       });
   });
}

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
   // Update text (as in original)
   for mut text in q_phase.iter_mut() {
       *text = Text::new(format!("Day {} {:02}:00", time.day, time.hour));
   }
   for mut text in q_stats.iter_mut() {
       *text = Text::new(format!("Thalers: {} | Rep: {}", player_stats.thalers, player_stats.reputation));
   }

   // Combat button logic
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
