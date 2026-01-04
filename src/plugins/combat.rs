use bevy::prelude::*;
use bevy::state::prelude::*;

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Health>()
            .register_type::<Attack>()
            .register_type::<Defense>()
            .register_type::<Speed>()
            .register_type::<ActionMeter>()
            .register_type::<MaterialType>()
            .register_type::<UnitType>()
            .register_type::<Team>()
            .add_systems(OnEnter(crate::plugins::core::GameState::NightPhase), spawn_combat_arena)
            .add_systems(OnExit(crate::plugins::core::GameState::NightPhase), cleanup_combat_ui)
            .add_systems(FixedUpdate, (tick_timer_system, combat_turn_system).chain().run_if(in_state(crate::plugins::core::GameState::NightPhase)))
            .add_systems(Update, update_combat_ui.run_if(in_state(crate::plugins::core::GameState::NightPhase)));
    }
}

// Marker Components for Combat UI
#[derive(Component)]
pub struct CombatLog;

#[derive(Component)]
pub struct CombatUnitUi;

fn cleanup_combat_ui(mut commands: Commands, q_root: Query<Entity, With<CombatUnitUi>>) {
    for e in q_root.iter() {
        commands.entity(e).despawn_recursive();
    }
}

#[derive(Component, Reflect, Debug, Clone, Copy, PartialEq, Eq)]
#[reflect(Component)]
pub enum Team {
    Player,
    Enemy,
}

// Systems
fn spawn_combat_arena(mut commands: Commands, q_existing: Query<Entity, With<CombatUnitUi>>) {
    // Clean up if re-entering (though ideally we track persistence)
    for e in q_existing.iter() {
        commands.entity(e).despawn_recursive();
    }

    // Spawn Arena UI Container
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            display: Display::Flex,
            justify_content: JustifyContent::SpaceEvenly,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Row,
            ..default()
        },
        BackgroundColor(Color::srgb(0.05, 0.0, 0.1)),
        CombatUnitUi, // Tag to cleanup later
    ))
    .with_children(|parent| {
        // Player Side
        parent.spawn((
            Node {
                width: Val::Px(200.0),
                height: Val::Px(300.0),
                border: UiRect::all(Val::Px(2.0)),
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BorderColor(Color::srgb(0.0, 0.0, 1.0)),
            BackgroundColor(Color::srgb(0.2, 0.2, 0.5)),
        ))
        .with_children(|p| {
             p.spawn((
                Text::new("Player Unit\nHuman\nHP: 100/100"),
                TextFont { font_size: 16.0, ..default() },
                TextColor(Color::WHITE),
             ));
        })
        .insert((
            Health { current: 100.0, max: 100.0 },
            Attack { value: 10.0 },
            Defense { value: 5.0 },
            Speed { value: 15.0 },
            ActionMeter::default(),
            UnitType::Human,
            MaterialType::Steel,
            Team::Player,
        ));

        // VS Text
        parent.spawn((
            Text::new("VS"),
            TextFont { font_size: 40.0, ..default() },
            TextColor(Color::srgb(1.0, 0.0, 0.0)),
        ));

        // Enemy Side
        parent.spawn((
            Node {
                width: Val::Px(200.0),
                height: Val::Px(300.0),
                border: UiRect::all(Val::Px(2.0)),
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BorderColor(Color::srgb(1.0, 0.0, 0.0)),
            BackgroundColor(Color::srgb(0.5, 0.2, 0.2)),
        ))
        .with_children(|p| {
             p.spawn((
                Text::new("Enemy Monster\nMonster\nHP: 150/150"),
                TextFont { font_size: 16.0, ..default() },
                TextColor(Color::WHITE),
             ));
        })
        .insert((
            Health { current: 150.0, max: 150.0 },
            Attack { value: 15.0 },
            Defense { value: 2.0 },
            Speed { value: 10.0 },
            ActionMeter::default(),
            UnitType::Monster,
            MaterialType::Flesh,
            Team::Enemy,
        ));
    });
}

fn update_combat_ui(
    q_units: Query<(&Health, &UnitType, &ActionMeter, &Children)>,
    mut q_text: Query<&mut Text>,
) {
    for (health, unit_type, meter, children) in q_units.iter() {
        for &child in children.iter() {
            if let Ok(mut text) = q_text.get_mut(child) {
                let type_name = match unit_type {
                    UnitType::Human => "Human",
                    UnitType::Monster => "Monster",
                    UnitType::Ethereal => "Ethereal",
                };
                **text = format!(
                    "{}\nHP: {:.0}/{:.0}\nMeter: {:.0}%",
                    type_name,
                    health.current,
                    health.max,
                    (meter.value / meter.threshold * 100.0).clamp(0.0, 100.0)
                );
            }
        }
    }
}

#[derive(Component, Reflect, Default, Debug, Clone, Copy)]
#[reflect(Component)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

#[derive(Component, Reflect, Default, Debug, Clone, Copy)]
#[reflect(Component)]
pub struct Attack {
    pub value: f32,
}

#[derive(Component, Reflect, Default, Debug, Clone, Copy)]
#[reflect(Component)]
pub struct Defense {
    pub value: f32,
}

#[derive(Component, Reflect, Default, Debug, Clone, Copy)]
#[reflect(Component)]
pub struct Speed {
    pub value: f32,
}

#[derive(Component, Reflect, Debug, Clone, Copy)]
#[reflect(Component)]
pub struct ActionMeter {
    pub value: f32,
    pub threshold: f32,
}

impl Default for ActionMeter {
    fn default() -> Self {
        Self {
            value: 0.0,
            threshold: 1000.0, // Default threshold from GDD example
        }
    }
}

#[derive(Component, Reflect, Default, Debug, Clone, Copy, PartialEq, Eq)]
#[reflect(Component)]
pub enum MaterialType {
    #[default]
    Steel,
    Silver,
    Flesh,
}

#[derive(Component, Reflect, Default, Debug, Clone, Copy, PartialEq, Eq)]
#[reflect(Component)]
pub enum UnitType {
    #[default]
    Human,
    Monster,
    Ethereal,
}

impl MaterialType {
    pub fn efficiency(&self, target: UnitType) -> f32 {
        match (self, target) {
            (MaterialType::Steel, UnitType::Human) => 1.5,
            (MaterialType::Steel, UnitType::Monster) => 0.8,
            (MaterialType::Steel, UnitType::Ethereal) => 0.0,

            (MaterialType::Silver, UnitType::Human) => 0.7,
            (MaterialType::Silver, UnitType::Monster) => 2.0,
            (MaterialType::Silver, UnitType::Ethereal) => 3.0,

            (MaterialType::Flesh, UnitType::Human) => 1.2,
            (MaterialType::Flesh, UnitType::Monster) => 1.2,
            (MaterialType::Flesh, UnitType::Ethereal) => 0.5,
        }
    }
}

pub fn calculate_damage(
    weapon_damage: f32,
    material: MaterialType,
    target_unit_type: UnitType,
    target_defense: f32,
) -> f32 {
    let modifier = material.efficiency(target_unit_type);
    let raw_damage = weapon_damage * modifier;

    if raw_damage >= target_defense {
        (2.0 * raw_damage - target_defense).max(0.0)
    } else {
        if target_defense > 0.0 {
            (raw_damage * raw_damage) / target_defense
        } else {
            raw_damage // Should not happen if defense is 0 (Raw >= Defense case covers it), but safety check
        }
    }
}

pub fn tick_timer_system(mut query: Query<(&Speed, &mut ActionMeter)>) {
    for (speed, mut meter) in query.iter_mut() {
        meter.value += speed.value;
    }
}

pub fn combat_turn_system(
    mut commands: Commands,
    mut q_units: Query<(Entity, &mut ActionMeter, &Attack, &Defense, &mut Health, &Team, &MaterialType, &UnitType)>,
    mut next_state: ResMut<NextState<crate::plugins::core::GameState>>,
) {
    // Collect all potential targets first to avoid borrow checker issues with double iteration
    let mut ready_units = Vec::new();
    for (entity, meter, _, _, _, team, _, _) in q_units.iter() {
        if meter.value >= meter.threshold {
            ready_units.push((entity, *team));
        }
    }

    for (attacker_entity, attacker_team) in ready_units {
        let (attacker_damage, attacker_material) =
            if let Ok((_, mut meter, attack, _, _, _, material, _)) = q_units.get_mut(attacker_entity) {
                 if meter.value < meter.threshold { continue; }
                 meter.value -= meter.threshold;
                 (attack.value, *material)
            } else {
                continue;
            };

        // Find target
        let mut target_entity_opt = None;
        let mut target_defense_val = 0.0;
        let mut target_unit_type_val = UnitType::Human;

        for (candidate_entity, _, _, defense, health, team, _, unit_type) in q_units.iter() {
            if *team != attacker_team && health.current > 0.0 {
                target_entity_opt = Some(candidate_entity);
                target_defense_val = defense.value;
                target_unit_type_val = *unit_type;
                break;
            }
        }

        if let Some(target_entity) = target_entity_opt {
             let damage = calculate_damage(attacker_damage, attacker_material, target_unit_type_val, target_defense_val);

             info!("Unit {:?} ({:?}) attacks {:?} for {:.1} damage!", attacker_entity, attacker_team, target_entity, damage);

             if let Ok((_, _, _, _, mut health, _, _, _)) = q_units.get_mut(target_entity) {
                 health.current -= damage;
                 if health.current <= 0.0 {
                     info!("Unit {:?} died!", target_entity);
                     commands.entity(target_entity).despawn_recursive();
                 }
             }
        }
    }

    // Check game over / victory conditions
    let mut player_alive = false;
    let mut enemy_alive = false;

    for (_, _, _, _, health, team, _, _) in q_units.iter() {
        if health.current > 0.0 {
            match team {
                Team::Player => player_alive = true,
                Team::Enemy => enemy_alive = true,
            }
        }
    }

    if !player_alive {
        info!("Player Defeated! Returning to City...");
        next_state.set(crate::plugins::core::GameState::DayPhase);
    } else if !enemy_alive {
        info!("Victory! Returning to City...");
        next_state.set(crate::plugins::core::GameState::DayPhase);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_material_efficiency() {
        assert_eq!(MaterialType::Steel.efficiency(UnitType::Human), 1.5);
        assert_eq!(MaterialType::Steel.efficiency(UnitType::Ethereal), 0.0);
        assert_eq!(MaterialType::Silver.efficiency(UnitType::Monster), 2.0);
        assert_eq!(MaterialType::Flesh.efficiency(UnitType::Human), 1.2);
    }

    #[test]
    fn test_damage_formula_high_pierce() {
        // RawDamage >= Defense
        // Formula: 2 * Raw - Defense
        let damage = 10.0;
        let modifier = 1.0;
        let defense = 5.0;
        // Raw = 10 * 1 = 10
        // Final = 2 * 10 - 5 = 15

        let calculated = calculate_damage(damage, MaterialType::Steel, UnitType::Human, defense);
        // Steel vs Human is 1.5x. Raw = 15. Final = 2*15 - 5 = 25.

        assert_eq!(calculated, 25.0);
    }

    #[test]
    fn test_damage_formula_low_pierce() {
        // RawDamage < Defense
        // Formula: Raw^2 / Defense
        let damage = 10.0;
        let modifier = 0.5; // Artificial modifier for easy math
        let defense = 20.0;

        // Let's use Steel (0.8) vs Monster
        let weapon_damage = 10.0;
        let material = MaterialType::Steel;
        let unit_type = UnitType::Monster;
        let defense = 20.0;

        // Raw = 10 * 0.8 = 8.0
        // 8 < 20
        // Final = 8^2 / 20 = 64 / 20 = 3.2

        let calculated = calculate_damage(weapon_damage, material, unit_type, defense);
        assert_eq!(calculated, 3.2);
    }

    #[test]
    fn test_action_meter_tick() {
        let mut app = App::new();
        app.add_systems(FixedUpdate, tick_timer_system);

        let entity = app.world_mut().spawn((
            Speed { value: 50.0 },
            ActionMeter { value: 0.0, threshold: 1000.0 },
        )).id();

        app.update(); // FixedUpdate might not run on single update without time setup, but let's see.
        // Actually, simulating FixedUpdate in test requires more setup.
        // Simplest is to just call the system logic or setup the schedule.

        // Let's just run the system manually on the world
        let mut schedule = Schedule::default();
        schedule.add_systems(tick_timer_system);
        schedule.run(app.world_mut());

        let meter = app.world().get::<ActionMeter>(entity).unwrap();
        assert_eq!(meter.value, 50.0);
    }
}
