use bevy::prelude::*;

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
            .add_systems(FixedUpdate, (tick_timer_system, combat_turn_system, fatigue_system).chain().run_if(in_state(crate::plugins::core::GameState::NightPhase)))
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
fn spawn_combat_arena(
    mut commands: Commands,
    q_existing: Query<Entity, With<CombatUnitUi>>,
    persistent_inventory: Res<crate::plugins::metagame::PersistentInventory>,
    item_db: Res<crate::plugins::items::ItemDatabase>,
) {
    // Clean up if re-entering (though ideally we track persistence)
    for e in q_existing.iter() {
        commands.entity(e).despawn_recursive();
    }

    let stats = crate::plugins::inventory::calculate_combat_stats(&persistent_inventory, &item_db);
    let base_hp = 100.0;
    let final_hp = base_hp + stats.health;

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
                Text::new(format!("Player Unit\nHuman\nHP: {:.0}/{:.0}", final_hp, final_hp)),
                TextFont { font_size: 16.0, ..default() },
                TextColor(Color::WHITE),
             ));
        })
        .insert((
            Health { current: final_hp, max: final_hp },
            Stamina { current: stats.stamina.max(10.0), max: stats.stamina.max(10.0), regen: 1.0 },
            StaminaCost { value: stats.stamina_cost.max(5.0) },
            Attack { value: stats.attack.max(1.0) },
            Defense { value: stats.defense },
            Speed { value: stats.speed.max(5.0) },
            Accuracy { value: 0.0 }, // Base accuracy handled in logic
            Block { value: 0.0 },
            Spikes { value: 0.0 },
            Vampirism { value: 0.0 },
            StatusEffects::default(),
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
            Stamina { current: 50.0, max: 50.0, regen: 2.0 },
            StaminaCost { value: 5.0 },
            Attack { value: 15.0 },
            Defense { value: 2.0 },
            Speed { value: 10.0 },
            Accuracy { value: 0.0 },
            Block { value: 0.0 },
            Spikes { value: 0.0 },
            Vampirism { value: 0.0 },
            StatusEffects::default(),
            ActionMeter::default(),
            UnitType::Monster,
            MaterialType::Flesh,
            Team::Enemy,
        ));
    });
}

fn update_combat_ui(
    q_units: Query<(&Health, &UnitType, &ActionMeter, &Stamina, &Children)>,
    mut q_text: Query<&mut Text>,
) {
    for (health, unit_type, meter, stamina, children) in q_units.iter() {
        for &child in children.iter() {
            if let Ok(mut text) = q_text.get_mut(child) {
                let type_name = match unit_type {
                    UnitType::Human => "Human",
                    UnitType::Monster => "Monster",
                    UnitType::Ethereal => "Ethereal",
                };
                **text = format!(
                    "{}\nHP: {:.0}/{:.0}\nStamina: {:.0}\nMeter: {:.0}%",
                    type_name,
                    health.current,
                    health.max,
                    stamina.current,
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

#[derive(Component, Reflect, Default, Debug, Clone, Copy)]
#[reflect(Component)]
pub struct Accuracy {
    pub value: f32, // Bonus accuracy
}

#[derive(Component, Reflect, Default, Debug, Clone, Copy)]
#[reflect(Component)]
pub struct Block {
    pub value: f32,
}

#[derive(Component, Reflect, Default, Debug, Clone, Copy)]
#[reflect(Component)]
pub struct Spikes {
    pub value: f32,
}

#[derive(Component, Reflect, Default, Debug, Clone, Copy)]
#[reflect(Component)]
pub struct Vampirism {
    pub value: f32,
}

#[derive(Component, Reflect, Default, Debug, Clone, Copy)]
#[reflect(Component)]
pub struct Stamina {
    pub current: f32,
    pub max: f32,
    pub regen: f32,
}

#[derive(Component, Reflect, Default, Debug, Clone)]
#[reflect(Component)]
pub struct StatusEffects {
    // Map of Effect Type -> Stacks
    // Simplified for now: just basic counters
    pub heat: u32,
    pub cold: u32,
    pub blind: u32,
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

// Fatigue System: Deals damage after 30 seconds
#[derive(Resource)]
pub struct BattleTimer(pub Timer);

impl Default for BattleTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(30.0, TimerMode::Once))
    }
}

pub fn fatigue_system(
    time: Res<Time>,
    mut timer: Local<BattleTimer>,
    mut q_health: Query<(&mut Health, &Team)>,
) {
    // If timer is finished, it means we are in fatigue phase
    if timer.0.tick(time.delta()).finished() {
         // Ramp up logic: Use elapsed time since finished?
         // For simplicity, just use constant damage but higher
         for (mut health, _) in q_health.iter_mut() {
             // 0.2 damage per tick (approx 12 dps)
             health.current -= 0.2;
         }
    }
}

// Full Combat Loop
pub fn combat_turn_system(
    mut commands: Commands,
    mut q_units: Query<(
        Entity,
        &mut ActionMeter,
        &Attack,
        &Defense,
        &mut Health,
        &Team,
        &MaterialType,
        &UnitType,
        &mut Stamina,
        &StaminaCost,
        &Accuracy,
        &Block,
        &Spikes,
        &Vampirism,
        &StatusEffects
    )>,
    mut next_state: ResMut<NextState<crate::plugins::core::GameState>>,
) {
    // 1. Identify attackers (threshold met)
    let mut attackers = Vec::new();
    for (entity, meter, ..) in q_units.iter() {
        if meter.value >= meter.threshold {
            attackers.push(entity);
        }
    }

    for attacker_entity in attackers {
        // We need to re-query to get mutable access safely without holding the whole world
        // But since we are iterating logic, we can try to extract values carefully.
        // Rust borrowing rules make this tricky with a single Query.
        // Pattern: Extract attacker data, then find target, then apply to target.

        let (attacker_damage, attacker_material, attacker_team, attacker_acc, attacker_vamp, attacker_blind_stacks) =
             if let Ok((_, mut meter, attack, _, _, team, material, _, mut stamina, cost, accuracy, _, _, vampirism, effects)) = q_units.get_mut(attacker_entity) {
                 if meter.value < meter.threshold { continue; } // Already spent?

                 // Stamina Check
                 let stamina_cost = cost.value;
                 if stamina.current < stamina_cost {
                     // Exhausted: Skip turn, reset meter slightly to try again later (or rest)
                     meter.value = 0.0;
                     // Recover some stamina instead of attacking
                     stamina.current += stamina.regen * 5.0;
                     info!("Unit {:?} is exhausted (Stamina: {:.1})!", attacker_entity, stamina.current);
                     continue;
                 }

                 stamina.current -= stamina_cost;
                 meter.value -= meter.threshold;

                 (attack.value, *material, *team, accuracy.value, vampirism.value, effects.blind)
            } else {
                continue;
            };

        // Find Target
        let mut target_data = None;
        // Simple targeting: First enemy found
        for (target_entity, _, _, defense, health, team, _, unit_type, _, _, _, block, spikes, _, _) in q_units.iter() {
            if *team != attacker_team && health.current > 0.0 {
                target_data = Some((target_entity, defense.value, *unit_type, block.value, spikes.value));
                break;
            }
        }

        if let Some((target_entity, target_def, target_type, target_block, target_spikes)) = target_data {
             // Accuracy Check
             // Base 100% + Accuracy - (Blind * 10%)
             let hit_chance = 100.0 + attacker_acc - (attacker_blind_stacks as f32 * 10.0);
             let roll = rand::thread_rng().gen_range(0.0..100.0);

             if roll > hit_chance {
                 info!("Unit {:?} missed target {:?} (Chance: {:.1}%)", attacker_entity, target_entity, hit_chance);
                 continue;
             }

             // Damage Calculation
             let raw_damage = calculate_damage(attacker_damage, attacker_material, target_type, target_def);

             // Block Reduction
             let blocked_damage = (raw_damage - target_block).max(0.0);

             info!("Unit {:?} hits {:?} for {:.1} (Raw: {:.1}, Blocked: {:.1})", attacker_entity, target_entity, blocked_damage, raw_damage, target_block);

             // Apply Damage to Target
             let mut damage_dealt = 0.0;
             if let Ok((_, _, _, _, mut t_health, _, _, _, _, _, _, _, _, _)) = q_units.get_mut(target_entity) {
                 t_health.current -= blocked_damage;
                 damage_dealt = blocked_damage;
                 if t_health.current <= 0.0 {
                     info!("Unit {:?} died!", target_entity);
                     commands.entity(target_entity).despawn_recursive();
                 }
             }

             // Apply Spikes (Reflect)
             if target_spikes > 0.0 {
                 if let Ok((_, _, _, _, mut a_health, _, _, _, _, _, _, _, _, _)) = q_units.get_mut(attacker_entity) {
                     a_health.current -= target_spikes;
                     info!("Unit {:?} took {:.1} spike damage!", attacker_entity, target_spikes);
                 }
             }

             // Apply Vampirism (Heal)
             if attacker_vamp > 0.0 && damage_dealt > 0.0 {
                 let heal = damage_dealt * (attacker_vamp / 100.0);
                 if let Ok((_, _, _, _, mut a_health, _, _, _, _, _, _, _, _, _)) = q_units.get_mut(attacker_entity) {
                     a_health.current = (a_health.current + heal).min(a_health.max);
                     info!("Unit {:?} healed {:.1} from vampirism!", attacker_entity, heal);
                 }
             }
        }
    }

    // Check Win Condition
    let mut player_alive = false;
    let mut enemy_alive = false;

    for (_, _, _, _, health, team, _, _, _, _, _, _, _, _) in q_units.iter() {
         if health.current > 0.0 {
            match team {
                Team::Player => player_alive = true,
                Team::Enemy => enemy_alive = true,
            }
        }
    }

    if !player_alive {
        info!("Player Defeated!");
        next_state.set(crate::plugins::core::GameState::DayPhase);
    } else if !enemy_alive {
        info!("Victory!");
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
