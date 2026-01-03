use bevy::prelude::*;
use bevy::utils::HashMap;
use serde::Deserialize;
use std::fs::File;
use std::io::Read;

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
    let path = "assets/items.ron";
    match File::open(path) {
        Ok(mut file) => {
            let mut content = String::new();
            if file.read_to_string(&mut content).is_ok() {
                match ron::from_str::<Vec<ItemDefinition>>(&content) {
                    Ok(items) => {
                        for item in items {
                            item_db.items.insert(item.id.clone(), item);
                        }
                        info!("ItemDatabase loaded with {} items from {}.", item_db.items.len(), path);
                    },
                    Err(e) => error!("Failed to parse items.ron: {}", e),
                }
            } else {
                error!("Failed to read items.ron");
            }
        },
        Err(e) => error!("Failed to open items.ron: {}. Make sure assets/items.ron exists.", e),
    }
}
