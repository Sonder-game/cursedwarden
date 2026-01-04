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
    #[serde(default)] // Allow omitting shape in JSON/RON if we generate it
    pub shape: Vec<IVec2>,
    #[allow(dead_code)]
    pub material: MaterialType,
    #[allow(dead_code)]
    pub item_type: ItemType,
    // Add stats later as needed
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
    // Add others as needed
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

    let mut items = vec![
        ItemDefinition {
            id: "steel_sword".to_string(),
            name: "Steel Sword".to_string(),
            width: 1,
            height: 2,
            shape: vec![], // Will be populated below
            material: MaterialType::Steel,
            item_type: ItemType::Weapon,
        },
        ItemDefinition {
            id: "silver_dagger".to_string(),
            name: "Silver Dagger".to_string(),
            width: 1,
            height: 1,
            shape: vec![],
            material: MaterialType::Silver,
            item_type: ItemType::Weapon,
        },
        ItemDefinition {
            id: "health_potion".to_string(),
            name: "Health Potion".to_string(),
            width: 1,
            height: 1,
            shape: vec![],
            material: MaterialType::Flesh,
            item_type: ItemType::Consumable,
        },
    ];

    // Auto-generate rectangular shapes if empty
    for item in items.iter_mut() {
        if item.shape.is_empty() {
            for y in 0..item.height {
                for x in 0..item.width {
                    item.shape.push(IVec2::new(x as i32, y as i32));
                }
            }
        }
    }

    for item in items {
        item_db.items.insert(item.id.clone(), item);
    }

    info!("ItemDatabase loaded with {} items.", item_db.items.len());
}
