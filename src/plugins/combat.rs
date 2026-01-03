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
            .register_type::<EntityType>()
            .add_systems(FixedUpdate, tick_timer_system);
    }
}

/// Helper for damage calculation
pub fn calculate_final_damage(raw_damage: f32, defense: f32) -> f32 {
    if raw_damage >= defense {
        2.0 * raw_damage - defense
    } else {
        if defense == 0.0 {
            // Avoid division by zero, though defense usually shouldn't be 0 if raw < defense (which implies raw < 0, impossible)
            // or if raw is 0 and defense is 0.
            // If defense is 0, raw >= defense is true (unless raw < 0).
            // So this branch is reachable if raw < defense.
            // If defense is 0, raw must be negative? Damage shouldn't be negative.
            // Let's safe guard.
            return 0.0;
        }
        (raw_damage * raw_damage) / defense
    }
}

#[derive(Component, Reflect, Default, Debug, Clone, Copy)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

#[derive(Component, Reflect, Default, Debug, Clone, Copy)]
pub struct Attack {
    pub value: f32,
}

#[derive(Component, Reflect, Default, Debug, Clone, Copy)]
pub struct Defense {
    pub value: f32,
}

#[derive(Component, Reflect, Default, Debug, Clone, Copy)]
pub struct Speed {
    pub value: f32,
}

#[derive(Component, Reflect, Debug, Clone, Copy)]
pub struct ActionMeter {
    pub current: f32,
    pub threshold: f32,
}

impl Default for ActionMeter {
    fn default() -> Self {
        Self {
            current: 0.0,
            threshold: 1000.0,
        }
    }
}

#[derive(Component, Reflect, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityType {
    #[default]
    Human,
    Monster,
    Ethereal,
}

#[derive(Component, Reflect, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Material {
    #[default]
    Steel,
    Silver,
    Flesh,
}

impl Material {
    pub fn get_modifier(&self, target_type: EntityType) -> f32 {
        match (self, target_type) {
            (Material::Steel, EntityType::Human) => 1.5,
            (Material::Steel, EntityType::Monster) => 0.8,
            (Material::Steel, EntityType::Ethereal) => 0.0,

            (Material::Silver, EntityType::Human) => 0.7,
            (Material::Silver, EntityType::Monster) => 2.0,
            (Material::Silver, EntityType::Ethereal) => 3.0,

            (Material::Flesh, EntityType::Human) => 1.2,
            (Material::Flesh, EntityType::Monster) => 1.2,
            (Material::Flesh, EntityType::Ethereal) => 0.5,
        }
    }
}

#[cfg(test)]
mod tests;

fn tick_timer_system(mut query: Query<(&Speed, &mut ActionMeter)>) {
    for (speed, mut meter) in query.iter_mut() {
        meter.current += speed.value;
    }
}
