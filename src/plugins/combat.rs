use bevy::prelude::*;
use crate::plugins::items::ItemDefinition;
use crate::plugins::items::ItemDatabase;
use crate::plugins::inventory::PersistentInventory;

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
            .add_systems(OnExit(crate::plugins::core::GameState::NightPhase), teardown_combat)
            .add_systems(FixedUpdate, (tick_timer_system, combat_turn_system, combat_end_system).chain().run_if(in_state(crate::plugins::core::GameState::NightPhase)))
            .add_systems(Update, update_combat_ui.run_if(in_state(crate::plugins::core::GameState::NightPhase)));
    }
}

// Marker Components for Combat UI
#[derive(Component)]
pub struct CombatLog;

#[derive(Component)]
pub struct CombatUnitUi;

#[derive(Component)]
pub struct CombatEntity; // Helper for cleanup

// Team Component
#[derive(Component, Reflect, Default, Debug, Clone, Copy, PartialEq, Eq)]
#[reflect(Component)]
pub enum Team {
    #[default]
    Player,
    Enemy,
}

// Systems
fn teardown_combat(mut commands: Commands, q_entities: Query<Entity, Or<(With<CombatUnitUi>, With<CombatEntity>)>>) {
    for e in q_entities.iter() {
        commands.entity(e).despawn_recursive();
    }
}

fn spawn_combat_arena(
    mut commands: Commands,
    persistent_inventory: Res<PersistentInventory>,
    item_db: Res<ItemDatabase>,
) {
    // Calculate Player Stats from Inventory
    let mut total_attack = 1.0; // Base attack
    let mut total_defense = 0.0;
    let mut total_speed = 15.0; // Base speed

    for saved_item in &persistent_inventory.items {
        if let Some(def) = item_db.items.get(&saved_item.item_id) {
            total_attack += def.attack;
            total_defense += def.defense;
            total_speed += def.speed;
        }
    }

    // Cap or sanitize values
    total_speed = total_speed.max(1.0);

    info!("Spawning Player with Stats: Atk {}, Def {}, Spd {}", total_attack, total_defense, total_speed);

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
        CombatUnitUi,
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
                Text::new(format!("Player Unit\nHuman\nHP: 100/100")),
                TextFont { font_size: 16.0, ..default() },
                TextColor(Color::WHITE),
             ));
        })
        .insert((
            Health { current: 100.0, max: 100.0 },
            Attack { value: total_attack },
            Defense { value: total_defense },
            Speed { value: total_speed },
            ActionMeter::default(),
            UnitType::Human,
            MaterialType::Steel, // Default for now, maybe derive from weapon?
            Team::Player,
            CombatEntity,
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
                Text::new("Enemy Monster\nMonster\nHP: 50/50"),
                TextFont { font_size: 16.0, ..default() },
                TextColor(Color::WHITE),
             ));
        })
        .insert((
            Health { current: 50.0, max: 50.0 }, // Lower HP for faster loops
            Attack { value: 5.0 },
            Defense { value: 1.0 },
            Speed { value: 10.0 },
            ActionMeter::default(),
            UnitType::Monster,
            MaterialType::Flesh,
            Team::Enemy,
            CombatEntity,
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
            threshold: 1000.0,
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
            raw_damage
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
    mut q_attackers: Query<(Entity, &mut ActionMeter, &Attack, &MaterialType, &Team)>,
    mut q_defenders: Query<(Entity, &mut Health, &Defense, &UnitType, &Team)>,
) {
    // Collect potential targets first to avoid borrow checker issues if we tried to iterate both simultaneously in a nested way
    // But since they are disjoint queries (mut vs mut), we can't do that easily if they overlap.
    // However, we need to find a target.

    // We iterate attackers.
    for (attacker_entity, mut meter, attack, material, attacker_team) in q_attackers.iter_mut() {
        if meter.value >= meter.threshold {
            // Find a valid target (Opposing team)
            let mut target: Option<(Entity, Mut<Health>, &Defense, &UnitType)> = None;

            // We iterate defenders. Since q_defenders overlaps with q_attackers (same entities),
            // we must be careful. Bevy queries with disjoint access are fine.
            // But here both are mut access to components on same entities?
            // q_attackers: Entity, ActionMeter (mut), Attack, Material, Team
            // q_defenders: Entity, Health (mut), Defense, UnitType, Team
            // The component sets are disjoint! (ActionMeter vs Health).
            // So we can iterate them safely.

            for (def_entity, def_health, def_defense, def_type, def_team) in q_defenders.iter_mut() {
                if attacker_team != def_team && def_health.current > 0.0 {
                    target = Some((def_entity, def_health, def_defense, def_type));
                    break; // Just pick first available for now
                }
            }

            if let Some((target_entity, mut target_health, target_defense, target_type)) = target {
                // Calculate Damage
                let damage = calculate_damage(attack.value, *material, *target_type, target_defense.value);

                info!("{:?} attacks {:?} for {} damage!", attacker_entity, target_entity, damage);

                target_health.current -= damage;

                // Reset meter
                meter.value -= meter.threshold;
            } else {
                // No valid target, wait (clamp to threshold)
                 meter.value = meter.threshold;
            }
        }
    }
}

pub fn combat_end_system(
    mut commands: Commands,
    q_units: Query<(&Health, &Team)>,
    mut next_state: ResMut<NextState<crate::plugins::core::GameState>>,
) {
    let mut player_alive = false;
    let mut enemy_alive = false;

    for (health, team) in q_units.iter() {
        if health.current > 0.0 {
            match team {
                Team::Player => player_alive = true,
                Team::Enemy => enemy_alive = true,
            }
        }
    }

    if !player_alive || !enemy_alive {
        // Combat Over
        info!("Combat Ended. Player Alive: {}, Enemy Alive: {}", player_alive, enemy_alive);

        // Brief delay could be added here, but for now instant transition
        next_state.set(crate::plugins::core::GameState::DayPhase);
    }
}
