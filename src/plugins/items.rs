use bevy::prelude::*;
use bevy::utils::HashMap;
use serde::Deserialize;

#[derive(Resource, Default)]
pub struct ItemDatabase {
    pub items: HashMap<String, ItemDefinition>,
}

#[derive(Debug, Clone, Deserialize, Component)]
pub struct ItemDefinition {
    pub id: String,
    pub name: String,
    pub width: u8,
    pub height: u8,
    #[allow(dead_code)]
    pub material: MaterialType,
    #[allow(dead_code)]
    pub item_type: ItemType,

    #[serde(default)]
    pub attack: f32,
    #[serde(default)]
    pub defense: f32,
    #[serde(default)]
    pub speed: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[allow(dead_code)]
pub enum MaterialType {
    Steel,
    Silver,
    Flesh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[allow(dead_code)]
pub enum ItemType {
    Weapon,
    Consumable,
    Ammo,
}

pub struct ItemsPlugin;

impl Plugin for ItemsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ItemDatabase>()
           .add_systems(Startup, load_items);
    }
}

fn load_items(mut item_db: ResMut<ItemDatabase>) {
    // For now, we mock the database loading.
    // In a real implementation, this would load from assets/items/*.ron

    let items = vec![
        ItemDefinition {
            id: "steel_sword".to_string(),
            name: "Steel Sword".to_string(),
            width: 1,
            height: 2,
            material: MaterialType::Steel,
            item_type: ItemType::Weapon,
            attack: 10.0,
            defense: 2.0,
            speed: 0.0,
        },
        ItemDefinition {
            id: "silver_dagger".to_string(),
            name: "Silver Dagger".to_string(),
            width: 1,
            height: 1,
            material: MaterialType::Silver,
            item_type: ItemType::Weapon,
            attack: 8.0,
            defense: 1.0,
            speed: 5.0,
        },
        ItemDefinition {
            id: "health_potion".to_string(),
            name: "Health Potion".to_string(),
            width: 1,
            height: 1,
            material: MaterialType::Flesh,
            item_type: ItemType::Consumable,
            attack: 0.0,
            defense: 0.0,
            speed: 0.0,
        },
        ItemDefinition {
            id: "heavy_shield".to_string(),
            name: "Heavy Shield".to_string(),
            width: 2,
            height: 2,
            material: MaterialType::Steel,
            item_type: ItemType::Weapon, // Or Armor
            attack: 2.0,
            defense: 10.0,
            speed: -5.0,
        },
    ];

    for item in items {
        item_db.items.insert(item.id.clone(), item);
    }

    info!("ItemDatabase loaded with {} items.", item_db.items.len());
}
