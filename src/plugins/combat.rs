use bevy::prelude::*;
use crate::plugins::core::GameState;
use crate::plugins::inventory::Item;
use crate::plugins::items::{ItemDefinition, ItemType, MaterialType as ItemMaterialType};

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
            .add_systems(OnEnter(GameState::NightPhase), (setup_combat_phase, apply_deferred, spawn_combat_ui).chain())
            .add_systems(FixedUpdate, (tick_timer_system, combat_turn_system).chain().run_if(in_state(GameState::NightPhase)))
            .add_systems(Update, update_combat_ui.run_if(in_state(GameState::NightPhase)))
            .add_systems(OnExit(GameState::NightPhase), teardown_combat);
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

#[derive(Component, Reflect, Default, Debug, Clone, Copy, PartialEq, Eq)]
#[reflect(Component)]
pub enum Team {
    #[default]
    Player,
    Enemy,
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

fn calculate_stats_from_inventory(
    q_items: &Query<&ItemDefinition, With<Item>>,
) -> (f32, f32, f32) {
    let mut attack = 5.0; // Base attack
    let mut defense = 0.0;
    let mut speed = 100.0;

    for item_def in q_items.iter() {
        match item_def.item_type {
            ItemType::Weapon => {
                match item_def.material {
                     ItemMaterialType::Steel => attack += 10.0,
                     ItemMaterialType::Silver => attack += 5.0,
                     ItemMaterialType::Flesh => attack += 8.0,
                }
            },
            ItemType::Consumable => {
                // Potions might give health? For now ignore.
            },
            _ => {}
        }
    }
    (attack, defense, speed)
}

fn setup_combat_phase(
    mut commands: Commands,
    q_items: Query<&ItemDefinition, With<Item>>,
) {
    // Calculate stats from inventory items
    let (player_attack, player_defense, player_speed) = calculate_stats_from_inventory(&q_items);

    // Spawn Player Unit
    commands.spawn((
        Name::new("Player"),
        Team::Player,
        UnitType::Human,
        Health { current: 100.0, max: 100.0 },
        Attack { value: player_attack },
        Defense { value: player_defense },
        Speed { value: player_speed },
        ActionMeter::default(),
        MaterialType::Steel,
        CombatEntity,
    ));

    // Spawn Enemy Unit
    commands.spawn((
        Name::new("Enemy"),
        Team::Enemy,
        UnitType::Monster,
        Health { current: 50.0, max: 50.0 },
        Attack { value: 5.0 },
        Defense { value: 0.0 },
        Speed { value: 80.0 },
        ActionMeter::default(),
        MaterialType::Flesh,
        CombatEntity,
    ));

    info!("Combat Phase Started: Player vs Enemy");
}

#[derive(Component)]
struct CombatUiRoot;

#[derive(Component)]
struct CombatEntity; // Tag for cleanup

#[derive(Component)]
struct CombatHealthText(Entity); // Entity it displays health for

fn teardown_combat(
    mut commands: Commands,
    q_combat_entities: Query<Entity, Or<(With<CombatEntity>, With<CombatUiRoot>)>>,
) {
    for entity in q_combat_entities.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn spawn_combat_ui(
    mut commands: Commands,
    q_units: Query<(Entity, &Name, &Team)>,
) {
    let mut player_entity = None;
    let mut enemy_entity = None;

    for (entity, _, team) in q_units.iter() {
        match team {
            Team::Player => player_entity = Some(entity),
            Team::Enemy => enemy_entity = Some(entity),
        }
    }

    if let (Some(player), Some(enemy)) = (player_entity, enemy_entity) {
        commands.spawn((
            CombatUiRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::FlexEnd,
                padding: UiRect::all(Val::Px(20.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
        )).with_children(|parent| {
            // Player Panel
            parent.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
            )).with_children(|p| {
                p.spawn((
                    Text::new("PLAYER"),
                    TextFont { font_size: 20.0, ..default() },
                    TextColor(Color::WHITE),
                ));
                p.spawn((
                    Text::new("Health: 100/100"),
                    TextFont { font_size: 16.0, ..default() },
                    TextColor(Color::srgb(0.0, 1.0, 0.0)),
                    CombatHealthText(player),
                ));
            });

            // Enemy Panel
            parent.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexEnd,
                    ..default()
                },
            )).with_children(|p| {
                p.spawn((
                    Text::new("ENEMY"),
                    TextFont { font_size: 20.0, ..default() },
                    TextColor(Color::WHITE),
                ));
                p.spawn((
                    Text::new("Health: 50/50"),
                    TextFont { font_size: 16.0, ..default() },
                    TextColor(Color::srgb(1.0, 0.0, 0.0)),
                    CombatHealthText(enemy),
                ));
            });
        });
    }
}

fn update_combat_ui(
    mut q_text: Query<(&mut Text, &CombatHealthText)>,
    q_health: Query<(&Health, &ActionMeter)>,
) {
    for (mut text, tracker) in q_text.iter_mut() {
        if let Ok((health, meter)) = q_health.get(tracker.0) {
            text.0 = format!("HP: {:.0}/{:.0} | AP: {:.0}", health.current, health.max, meter.value);
        }
    }
}

pub fn combat_turn_system(
    mut commands: Commands,
    mut q_units: Query<(Entity, &mut ActionMeter, &Attack, &MaterialType, &UnitType, &Team)>,
    mut q_targets: Query<(Entity, &mut Health, &Defense, &UnitType, &Team)>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let mut dead_entities = Vec::new();

    // Iterate all units that can attack
    for (attacker_entity, mut meter, attack, material, _attacker_type, attacker_team) in q_units.iter_mut() {
        if meter.value >= meter.threshold {
            // Find a target from the opposing team
            let target = q_targets.iter_mut()
                .filter(|(e, _, _, _, target_team)| *e != attacker_entity && *target_team != attacker_team)
                .next();

            if let Some((target_entity, mut target_health, target_defense, target_type, _)) = target {
                // Calculate Damage
                let damage = calculate_damage(attack.value, *material, *target_type, target_defense.value);

                info!("{:?} attacks {:?} for {} damage!", attacker_team, target_entity, damage);

                target_health.current -= damage;

                // Reset meter
                meter.value -= meter.threshold;

                // Check Death
                if target_health.current <= 0.0 {
                    info!("Unit {:?} died!", target_entity);
                    dead_entities.push(target_entity);

                    // If Enemy died, Player wins phase?
                    // If Player died, Game Over?
                    // For now, just transition back to Day if Enemy dies, or GameOver if Player dies.
                    // But we can't check Team of dead entity easily here without another query or looking up component.
                    // We'll handle state transition in the next frame or cleanup system.
                }
            } else {
                // No valid targets?
                meter.value = meter.threshold;
            }
        }
    }

    // Cleanup dead
    for entity in dead_entities {
        // Determine team before despawn? We have the entity ID.
        // We can re-query or just check if it was player.
        // For simplicity, let's just query team of dying entity.
        if let Ok((_, _, _, _, team)) = q_targets.get(entity) {
             match team {
                 Team::Player => {
                     info!("Player died! Game Over.");
                     next_state.set(GameState::GameOver);
                 },
                 Team::Enemy => {
                     info!("Enemy Defeated! Victory!");
                     next_state.set(GameState::DayPhase);
                 }
             }
        }
        commands.entity(entity).despawn_recursive();
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
