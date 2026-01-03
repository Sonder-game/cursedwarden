use bevy::prelude::*;

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Health>()
            .register_type::<Attack>()
            .register_type::<Defense>()
            .register_type::<Speed>()
            .register_type::<ActionMeter>()
            .register_type::<Material>()
            .register_type::<UnitType>()
            .add_systems(FixedUpdate, tick_timer_system);
    }
}

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct Attack {
    pub damage: f32,
}

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct Defense {
    pub armor: f32,
}

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct Speed {
    pub value: f32,
}

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct ActionMeter {
    pub current: f32,
    pub threshold: f32,
}

#[derive(Component, Reflect, Default, PartialEq, Clone, Copy)]
#[reflect(Component)]
pub enum Material {
    #[default]
    Steel,
    Silver,
    Flesh,
}

#[derive(Component, Reflect, Default, PartialEq, Clone, Copy)]
#[reflect(Component)]
pub enum UnitType {
    #[default]
    Human,
    Monster,
    Ethereal,
}

/// Increases ActionMeter by Speed every tick
fn tick_timer_system(mut query: Query<(&mut ActionMeter, &Speed)>) {
    for (mut meter, speed) in query.iter_mut() {
        meter.current += speed.value;
    }
}

/// Calculates the raw damage multiplier based on Material vs UnitType
pub fn get_material_modifier(material: Material, target_type: UnitType) -> f32 {
    match (material, target_type) {
        // Steel
        (Material::Steel, UnitType::Human) => 1.5,
        (Material::Steel, UnitType::Monster) => 0.8,
        (Material::Steel, UnitType::Ethereal) => 0.0,
        // Silver
        (Material::Silver, UnitType::Human) => 0.7,
        (Material::Silver, UnitType::Monster) => 2.0,
        (Material::Silver, UnitType::Ethereal) => 3.0,
        // Flesh
        (Material::Flesh, UnitType::Human) => 1.2,
        (Material::Flesh, UnitType::Monster) => 1.2,
        (Material::Flesh, UnitType::Ethereal) => 0.5,
    }
}

/// Calculates final damage using the hybrid formula:
/// RawDamage = WeaponDamage * MaterialModifier
/// If RawDamage >= Defense: Final = 2 * Raw - Defense
/// If RawDamage < Defense: Final = Raw^2 / Defense
pub fn calculate_damage(
    weapon_damage: f32,
    material: Material,
    target_type: UnitType,
    defense: f32,
) -> f32 {
    let modifier = get_material_modifier(material, target_type);
    let raw_damage = weapon_damage * modifier;

    if raw_damage >= defense {
        2.0 * raw_damage - defense
    } else {
        if defense == 0.0 {
            // Edge case: should not happen with non-zero defense logic, but to be safe
            raw_damage
        } else {
            (raw_damage * raw_damage) / defense
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_material_matrix() {
        // Steel
        assert_eq!(get_material_modifier(Material::Steel, UnitType::Human), 1.5);
        assert_eq!(get_material_modifier(Material::Steel, UnitType::Monster), 0.8);
        assert_eq!(get_material_modifier(Material::Steel, UnitType::Ethereal), 0.0);

        // Silver
        assert_eq!(get_material_modifier(Material::Silver, UnitType::Monster), 2.0);

        // Flesh
        assert_eq!(get_material_modifier(Material::Flesh, UnitType::Human), 1.2);
    }

    #[test]
    fn test_damage_formula_penetration() {
        // RawDamage >= Defense
        // Weapon: 10, Material: Steel (vs Human 1.5x) -> Raw = 15
        // Defense: 10
        // Expected: 2 * 15 - 10 = 20
        let damage = calculate_damage(10.0, Material::Steel, UnitType::Human, 10.0);
        assert_eq!(damage, 20.0);
    }

    #[test]
    fn test_damage_formula_glance() {
        // RawDamage < Defense
        // Weapon: 10, Material: Steel (vs Monster 0.8x) -> Raw = 8
        // Defense: 10
        // Expected: 8^2 / 10 = 6.4
        let damage = calculate_damage(10.0, Material::Steel, UnitType::Monster, 10.0);
        assert!((damage - 6.4).abs() < f32::EPSILON);
    }

    #[test]
    fn test_zero_defense() {
       // RawDamage = 15 (10 * 1.5), Defense = 0
       // 2 * 15 - 0 = 30
       let damage = calculate_damage(10.0, Material::Steel, UnitType::Human, 0.0);
       assert_eq!(damage, 30.0);
    }

    #[test]
    fn test_high_defense() {
        // RawDamage = 8 (10 * 0.8), Defense = 100
        // 64 / 100 = 0.64
        let damage = calculate_damage(10.0, Material::Steel, UnitType::Monster, 100.0);
        assert!((damage - 0.64).abs() < f32::EPSILON);
    }
}
