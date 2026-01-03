use bevy::prelude::*;
use bevy::utils::HashMap;
use serde::Deserialize;
use std::fs::File;
use std::io::Read;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum MaterialType {
    Steel,
    Silver,
    Flesh,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ItemDefinition {
    pub name: String,
    pub width: i32,
    pub height: i32,
    pub material: MaterialType,
}

#[derive(Resource, Deserialize, Default)]
pub struct ItemDatabase {
    pub items: Vec<ItemDefinition>,
}

pub struct ItemsPlugin;

impl Plugin for ItemsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ItemDatabase>()
           .add_systems(Startup, load_item_database);
    }
}

fn load_item_database(mut item_db: ResMut<ItemDatabase>) {
    // In a real project with Bevy, we should use AssetServer for async loading.
    // However, for simplicity and to satisfy the immediate requirement of loading from .ron,
    // we will do a blocking load here or use a simple std::fs approach.
    // Given the constraints and "quick start", blocking load in Startup is acceptable for small data.

    let path = "assets/items.ron";
    match File::open(path) {
        Ok(mut file) => {
            let mut content = String::new();
            if file.read_to_string(&mut content).is_ok() {
                match ron::from_str::<ItemDatabase>(&content) {
                    Ok(db) => {
                        *item_db = db;
                        info!("Item Database loaded: {} items", item_db.items.len());
                    },
                    Err(e) => error!("Failed to parse items.ron: {}", e),
                }
            } else {
                error!("Failed to read items.ron");
            }
        },
        Err(e) => error!("Failed to open items.ron: {}", e),
    }
}
