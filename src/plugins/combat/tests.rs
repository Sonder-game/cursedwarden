use super::*;

#[test]
fn test_damage_formula_high_raw() {
    let raw = 10.0;
    let defense = 5.0;
    // Raw >= Defense: 2 * 10 - 5 = 15
    assert_eq!(calculate_final_damage(raw, defense), 15.0);
}

#[test]
fn test_damage_formula_low_raw() {
    let raw = 4.0;
    let defense = 8.0;
    // Raw < Defense: 4^2 / 8 = 16 / 8 = 2
    assert_eq!(calculate_final_damage(raw, defense), 2.0);
}

#[test]
fn test_damage_formula_equal() {
    let raw = 5.0;
    let defense = 5.0;
    // Raw >= Defense: 2 * 5 - 5 = 5
    assert_eq!(calculate_final_damage(raw, defense), 5.0);
}

#[test]
fn test_material_modifiers() {
    let steel = Material::Steel;
    assert_eq!(steel.get_modifier(EntityType::Human), 1.5);
    assert_eq!(steel.get_modifier(EntityType::Monster), 0.8);
    assert_eq!(steel.get_modifier(EntityType::Ethereal), 0.0);

    let silver = Material::Silver;
    assert_eq!(silver.get_modifier(EntityType::Human), 0.7);
    assert_eq!(silver.get_modifier(EntityType::Monster), 2.0);
    assert_eq!(silver.get_modifier(EntityType::Ethereal), 3.0);

    let flesh = Material::Flesh;
    assert_eq!(flesh.get_modifier(EntityType::Human), 1.2);
    assert_eq!(flesh.get_modifier(EntityType::Monster), 1.2);
    assert_eq!(flesh.get_modifier(EntityType::Ethereal), 0.5);
}

#[test]
fn test_action_meter_tick() {
    let mut app = App::new();
    app.add_plugins(CombatPlugin);

    let entity = app.world_mut().spawn((
        Speed { value: 10.0 },
        ActionMeter { current: 0.0, threshold: 100.0 },
    )).id();

    app.update(); // Initialize systems

    // We registered `tick_timer_system` on `FixedUpdate`.
    // To test it, we need to advance the fixed time or run the schedule manually.
    // However, simplest way in unit test is to run the schedule.
    app.world_mut().run_schedule(FixedUpdate);

    let meter = app.world().get::<ActionMeter>(entity).unwrap();
    assert_eq!(meter.current, 10.0);

    app.world_mut().run_schedule(FixedUpdate);
    let meter = app.world().get::<ActionMeter>(entity).unwrap();
    assert_eq!(meter.current, 20.0);
}
