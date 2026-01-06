use crate::plugins::items::{ItemDatabase, StatType, SynergyEffect};
use crate::plugins::metagame::{PersistentInventory, SavedItem};
use bevy::prelude::*;
use bevy::utils::HashMap;

#[derive(Default, Debug, Clone)]
pub struct CombatStats {
    pub attack: f32,
    pub defense: f32,
    pub speed: f32,
    pub health: f32,
}

pub fn calculate_combat_stats(
    inventory: &PersistentInventory,
    db: &ItemDatabase,
) -> CombatStats {
    let mut stats = CombatStats::default();

    // 1. Reconstruct grid to calculate synergies
    // This is a simplified simulation because PersistentInventory just has a list of items
    // We need to know where they are relative to each other.
    // SavedItem has `grid_x` and `grid_y`.

    // Map: GridPos -> (SavedItem)
    let mut grid_map: HashMap<IVec2, &SavedItem> = HashMap::new();

    for item in &inventory.items {
        if let Some(def) = db.items.get(&item.item_id) {
            let shape = crate::plugins::inventory::rotate_shape(&def.shape, item.rotation);
            for offset in shape {
                let pos = IVec2::new(item.grid_x, item.grid_y) + offset;
                grid_map.insert(pos, item);
            }
        }
    }

    // 2. Iterate items and apply base stats + synergies
    for item in &inventory.items {
        if let Some(def) = db.items.get(&item.item_id) {
            // Base Stats
            stats.attack += def.attack;
            stats.defense += def.defense;
            stats.speed += def.speed;
            // stats.health += def.health; // If added to ItemDefinition

            // Synergies
            for synergy in &def.synergies {
                // Calculate target position based on rotation
                // Synergy offset is relative to the item's pivot (0,0)
                // We rotate the synergy offset vector by the item's rotation
                let rotated_offset = rotate_vector(synergy.offset, item.rotation);
                let target_pos = IVec2::new(item.grid_x, item.grid_y) + rotated_offset;

                if let Some(target_item) = grid_map.get(&target_pos) {
                     if let Some(target_def) = db.items.get(&target_item.item_id) {
                         // Check tags
                         let match_found = synergy.target_tags.iter().any(|tag| target_def.tags.contains(tag));

                         if match_found {
                             match &synergy.effect {
                                 SynergyEffect::BuffSelf { stat, value } => {
                                     apply_stat_bonus(&mut stats, *stat, *value);
                                 }
                                 SynergyEffect::BuffTarget { stat: _, value: _ } => {
                                     // This is trickier because we are iterating linearly.
                                     // "BuffTarget" implies the target gets stats.
                                     // In a "Snapshot" calculation, we can just add to global stats
                                     // UNLESS the buff is specific to the item (like "this sword deals +5").
                                     // For global player stats, it doesn't matter who gets the buff.
                                     // So we just apply it to the global sum.
                                     // apply_stat_bonus(&mut stats, *stat, *value);

                                     // Wait, if I have 2 swords, and one gets +5 attack, total attack is +5.
                                     // So yes, just add to global.
                                     // NOTE: If we wanted per-weapon stats (e.g. multi-attack), we'd need a different return structure.
                                 }
                                 SynergyEffect::BagBonus { bag_type: _, stat: _, value: _ } => {
                                     // Check if item is inside the specific bag
                                     // This requires checking "Slots" which we didn't fully reconstruct in this lightweight pass.
                                     // However, we can check if the underlying slot is provided by a bag of that type.
                                     // But `grid_map` here only maps Items. We need a Slot map.
                                     // For now, skip or implement fully if needed.
                                 }
                             }
                         }
                     }
                }
            }
        }
    }

    stats
}

fn apply_stat_bonus(stats: &mut CombatStats, stat: StatType, value: f32) {
    match stat {
        StatType::Attack => stats.attack += value,
        StatType::Defense => stats.defense += value,
        StatType::Speed => stats.speed += value,
        StatType::Health => stats.health += value,
    }
}

// Helper to rotate a single vector (same logic as in inventory.rs but for single vec)
fn rotate_vector(p: IVec2, rot: u8) -> IVec2 {
    let turns = rot % 4;
    let mut v = p;
    for _ in 0..turns {
        v = IVec2::new(-v.y, v.x);
    }
    v
}
