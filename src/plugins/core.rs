use bevy::prelude::*;

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
           .add_sub_state::<DaySubState>()
           .add_systems(OnEnter(GameState::AssetLoading), finish_loading)
           .add_systems(OnEnter(GameState::GameOver), setup_game_over_ui)
           .add_systems(Update, game_over_input_system.run_if(in_state(GameState::GameOver)));
    }
}

fn finish_loading(mut next_state: ResMut<NextState<GameState>>) {
    info!("Assets loaded (mock). Transitioning to EveningPhase.");
    next_state.set(GameState::EveningPhase);
}

fn setup_game_over_ui(mut commands: Commands) {
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Column,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.9)),
    )).with_children(|parent| {
        parent.spawn((
            Text::new("GAME OVER"),
            TextFont { font_size: 40.0, ..default() },
            TextColor(Color::srgb(1.0, 0.0, 0.0)),
        ));
        parent.spawn((
            Text::new("Press R to Restart"),
            TextFont { font_size: 20.0, ..default() },
            TextColor(Color::WHITE),
        ));
    });
}

fn game_over_input_system(
    input: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if input.just_pressed(KeyCode::KeyR) {
        // In a real app we might want to reset resources, but for now just go to EveningPhase to start over
        next_state.set(GameState::EveningPhase);
    }
}

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum GameState {
   #[default]
   AssetLoading,
   #[allow(dead_code)]
   MainMenu,
   DayPhase,
   EveningPhase,          // Inventory management
   NightPhase,            // Auto-battle
   #[allow(dead_code)]
   EventResolution,       // Dialogs
   GameOver,
}

#[derive(SubStates, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
#[source(GameState = GameState::DayPhase)]
pub enum DaySubState {
   #[default]
   Idle,
   #[allow(dead_code)]
   Trading,
   #[allow(dead_code)]
   MapTravel,
}
