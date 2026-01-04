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

    // Use the BattleBridge to get snapshot
    let (stats, battle_items) = crate::plugins::inventory::create_battle_snapshot(&persistent_inventory, &item_db);

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
        // Player Side (Container)
        let mut player_entity_cmds = parent.spawn((
            Node {
                width: Val::Px(300.0), // Wider to hold items
                height: Val::Px(500.0),
                border: UiRect::all(Val::Px(2.0)),
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexStart,
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            BorderColor(Color::srgb(0.0, 0.0, 1.0)),
            BackgroundColor(Color::srgb(0.2, 0.2, 0.5)),
        ));

        player_entity_cmds.with_children(|p| {
             // Hero Stats
             p.spawn((
                Text::new(format!("Player\nHP: {:.0}/{:.0}\nDef: {:.0}", final_hp, final_hp, stats.defense)),
                TextFont { font_size: 16.0, ..default() },
                TextColor(Color::WHITE),
             ));
        })
        .insert((
            Health { current: final_hp, max: final_hp },
            // Player entity itself doesn't attack, items do. But we keep components for safety/targeting.
            Attack { value: 0.0 },
            Defense { value: stats.defense },
            Speed { value: 0.0 },
            // Player doesn't need ActionMeter, but components might expect it.
            // We'll leave it but not increment it.
            ActionMeter::default(),
            UnitType::Human,
            MaterialType::Steel,
            Team::Player,
            Stamina { current: 10.0, max: 10.0 }, // Base Stamina
        ));

        // Spawn Active Battle Items as Children
        player_entity_cmds.with_children(|p| {
             p.spawn(Node {
                 height: Val::Px(20.0),
                 ..default()
             }); // Spacer

             for item in battle_items {
                 p.spawn((
                     Node {
                         width: Val::Percent(90.0),
                         height: Val::Px(40.0),
                         margin: UiRect::bottom(Val::Px(5.0)),
                         padding: UiRect::all(Val::Px(5.0)),
                         display: Display::Flex,
                         flex_direction: FlexDirection::Column,
                         justify_content: JustifyContent::Center,
                         ..default()
                     },
                     BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
                 ))
                 .with_children(|item_ui| {
                     item_ui.spawn((
                         Text::new(format!("{} (Dmg: {:.1})", item.name, item.damage)),
                         TextFont { font_size: 14.0, ..default() },
                         TextColor(Color::WHITE),
                     ));
                     item_ui.spawn((
                         Text::new("Loading..."),
                         TextFont { font_size: 12.0, ..default() },
                         TextColor(Color::srgb(0.8, 0.8, 1.0)),
                         CombatLog, // Tag to update this text
                     ));
                 })
                 .insert((
                     Attack { value: item.damage },
                     Speed { value: item.cooldown }, // Uses cooldown logic
                     ActionMeter { value: 0.0, threshold: 1000.0 },
                     // Convert items::MaterialType to combat::MaterialType
                     match item.material {
                         crate::plugins::items::MaterialType::Steel => MaterialType::Steel,
                         crate::plugins::items::MaterialType::Silver => MaterialType::Silver,
                         crate::plugins::items::MaterialType::Flesh => MaterialType::Flesh,
                     },
                     Team::Player, // Belongs to player team
                     CombatItemTag {
                         accuracy: item.accuracy,
                         stamina_cost: item.stamina_cost
                     }
                 ));
             }
        });

        // VS Text
        parent.spawn((
            Text::new("VS"),
            TextFont { font_size: 40.0, ..default() },
            TextColor(Color::srgb(1.0, 0.0, 0.0)),
        ));

        // Enemy Side
        parent.spawn((
            Node {
                width: Val::Px(300.0),
                height: Val::Px(500.0),
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
    q_units: Query<(&Health, &UnitType, &Defense, Option<&Stamina>, &Children)>, // Player/Enemy Main Units
    q_items: Query<(&ActionMeter, &Children), With<CombatItemTag>>, // Items
    mut q_text: Query<&mut Text>,
) {
    // Update Main Units
    for (health, unit_type, defense, stamina, children) in q_units.iter() {
        // Find the text child directly under the unit
        for &child in children.iter() {
             // We only want to update the main label, which is usually the first text child.
             // But items are also children. We can distinguish by looking if the child has children?
             // Or simpler: The first child of the Unit is the Text.

             if let Ok(mut text) = q_text.get_mut(child) {
                 if text.as_str().contains("HP:") { // Hacky check to ensure we update the stat block
                     let type_name = match unit_type {
                        UnitType::Human => "Human",
                        UnitType::Monster => "Monster",
                        UnitType::Ethereal => "Ethereal",
                    };
                    let stamina_str = if let Some(s) = stamina { format!("\nStamina: {:.1}", s.current) } else { "".to_string() };

                    **text = format!(
                        "{}\nHP: {:.0}/{:.0}\nDef: {:.0}{}",
                        type_name,
                        health.current,
                        health.max,
                        defense.value,
                        stamina_str
                    );
                 }
             }
        }
    }

    // Update Items
    for (meter, children) in q_items.iter() {
        for &child in children.iter() {
             if let Ok(mut text) = q_text.get_mut(child) {
                 // The item has 2 text children, one static name, one dynamic status.
                 // We tagged dynamic status with CombatLog.
                 // Wait, we can't query CombatLog here easily without traversing.
                 // Let's just check if it's the loading/meter text.
                 if text.as_str().contains("Meter") || text.as_str().contains("Loading") || text.as_str().contains("%") {
                     **text = format!("Meter: {:.0}%", (meter.value / meter.threshold * 100.0).clamp(0.0, 100.0));
                 }
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
    pub value: f32, // For items, this is speed/cooldown rate
}

#[derive(Component, Reflect, Debug, Clone, Copy)]
#[reflect(Component)]
pub struct ActionMeter {
    pub value: f32,
    pub threshold: f32,
}

#[derive(Component, Reflect, Default, Debug, Clone, Copy)]
#[reflect(Component)]
pub struct Stamina {
    pub current: f32,
    pub max: f32,
}

#[derive(Component, Reflect, Default, Debug, Clone, Copy)]
#[reflect(Component)]
pub struct CombatItemTag {
    pub accuracy: f32,
    pub stamina_cost: f32,
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

pub fn tick_timer_system(
    mut q_meters: Query<(&Speed, &mut ActionMeter)>,
    mut q_stamina: Query<&mut Stamina>,
) {
    // Tick meters
    for (speed, mut meter) in q_meters.iter_mut() {
        meter.value += speed.value;
    }

    // Regen stamina
    for mut stamina in q_stamina.iter_mut() {
        if stamina.current < stamina.max {
            stamina.current = (stamina.current + 0.05).min(stamina.max); // ~3 stamina per sec
        }
    }
}

pub fn combat_turn_system(
    mut commands: Commands,
    mut q_movers: Query<(Entity, &mut ActionMeter, &Attack, &Speed, &Team, Option<&MaterialType>, Option<&CombatItemTag>, Option<&Parent>)>,
    mut q_targets: Query<(Entity, &Team, &mut Health, &Defense, &UnitType)>,
    mut q_parents: Query<&mut Stamina>,
    mut next_state: ResMut<NextState<crate::plugins::core::GameState>>,
) {
    // Identify units ready to act
    // Note: q_movers includes both Main Units (like Enemy) and Item Entities (Player Weapons).
    // Enemy Unit has Attack, Speed, Team, Material, etc.
    // Item Entity has Attack, Speed, Team, Material, CombatItemTag.

    let mut actions = Vec::new();

    for (entity, meter, attack, _, team, material, tag, parent) in q_movers.iter() {
        if meter.value >= meter.threshold {
            // Copy all data to avoid borrowing q_movers
            actions.push((entity, *team, attack.value, material.copied(), tag.copied(), parent.map(|p| p.get())));
        }
    }

    for (entity, team, damage, material_opt, tag_opt, parent_entity_opt) in actions {

        // Check Stamina if item
        if let Some(tag) = tag_opt {
            if let Some(parent_entity) = parent_entity_opt {
                if let Ok(mut stamina) = q_parents.get_mut(parent_entity) {
                    if stamina.current < tag.stamina_cost {
                        // Fizzle / Wait for stamina
                        // For now, let's just not attack but keep the meter full?
                        // Or burn meter and do nothing?
                        // Backpack Battles slows down attack if no stamina.
                        // Let's just return early (skip this attack)
                        continue;
                    }
                    stamina.current -= tag.stamina_cost;
                }
            }
        }

        // Reset Meter
        if let Ok((_, mut meter, _, _, _, _, _, _)) = q_movers.get_mut(entity) {
             meter.value -= meter.threshold;
        }

        // Find Target
        let mut target = None;
        for (t_entity, t_team, _, t_def, t_type) in q_targets.iter() {
            if *t_team != team {
                target = Some((t_entity, t_def.value, *t_type));
                break; // Attack first valid target (1v1)
            }
        }

        if let Some((target_entity, target_def, target_type)) = target {
            let material = material_opt.unwrap_or(MaterialType::Steel); // Default
            let final_damage = calculate_damage(damage, material, target_type, target_def);

            info!("Entity {:?} attacks {:?} for {:.1} damage!", entity, target_entity, final_damage);

            if let Ok((_, _, mut health, _, _)) = q_targets.get_mut(target_entity) {
                health.current -= final_damage;
                if health.current <= 0.0 {
                    // commands.entity(target_entity).despawn_recursive(); // Don't despawn immediately, just mark dead or let cleanup handle
                }
            }
        }
    }

    // Check Game Over
    let mut player_hp = 0.0;
    let mut enemy_hp = 0.0;

    for (_, team, health, _, _) in q_targets.iter() {
        if health.current > 0.0 {
             match team {
                 Team::Player => player_hp = health.current,
                 Team::Enemy => enemy_hp = health.current,
             }
        }
    }

    if player_hp <= 0.0 {
        info!("Player Defeated! Returning to City...");
        next_state.set(crate::plugins::core::GameState::DayPhase);
    } else if enemy_hp <= 0.0 {
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
