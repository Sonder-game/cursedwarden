use bevy::prelude::*;
use bevy::utils::HashMap;
use serde::Deserialize;

#[derive(Resource, Default)]
pub struct ItemDatabase {
    pub items: HashMap<String, ItemDefinition>,
    pub recipes: Vec<RecipeDefinition>,
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

    #[serde(default)]
    pub rarity: ItemRarity,

    #[serde(default)]
    pub price: u32,

    #[serde(default)]
    pub tags: Vec<ItemTag>,

    #[serde(default)]
    pub synergies: Vec<SynergyDefinition>,

    // Base Stats
    #[serde(default)]
    pub attack: f32,
    #[serde(default)]
    pub defense: f32,
    #[serde(default)]
    pub speed: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RecipeDefinition {
    pub result_item_id: String,
    pub ingredients: Vec<RecipeIngredient>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct RecipeIngredient {
    pub item_id: String,
    pub is_catalyst: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Hash, PartialOrd, Ord)]
pub enum ItemRarity {
    Common,
    Rare,
    Epic,
    Legendary,
    Godly,
    Unique,
}

impl Default for ItemRarity {
    fn default() -> Self {
        Self::Common
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Hash)]
pub enum ItemTag {
    Weapon,
    Potion,
    Food,
    Magic,
    Valuable,
    Armor,
    Accessory,
    // Add more as needed
}

#[derive(Debug, Clone, Deserialize)]
pub struct SynergyDefinition {
    // Relative coordinate from item pivot (0,0)
    // Note: This needs to rotate with the item
    pub offset: IVec2,
    // If the item at 'offset' has ANY of these tags, the effect triggers
    pub target_tags: Vec<ItemTag>,
    pub effect: SynergyEffect,
}

#[derive(Debug, Clone, Deserialize)]
pub enum SynergyEffect {
    // Apply stat bonus to the TARGET item
    BuffTarget {
        stat: StatType,
        value: f32,
    },
    // Apply stat bonus to SELF if target is found
    BuffSelf {
        stat: StatType,
        value: f32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum StatType {
    Attack,
    Defense,
    Speed,
    Health,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[allow(dead_code)]
pub enum MaterialType {
    Steel,
    Silver,
    Flesh,
    Wood,
    Crystal,
    Fabric,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[allow(dead_code)]
pub enum ItemType {
    Weapon,
    Consumable,
    Ammo,
    Armor,
    Accessory,
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
            id: "wooden_sword".to_string(),
            name: "Wooden Sword".to_string(),
            width: 1, height: 2, shape: vec![],
            material: MaterialType::Wood,
            item_type: ItemType::Weapon,
            rarity: ItemRarity::Common,
            price: 3,
            tags: vec![ItemTag::Weapon],
            synergies: vec![],
            attack: 4.0, defense: 0.0, speed: 0.0,
        },
        ItemDefinition {
            id: "steel_sword".to_string(),
            name: "Steel Sword".to_string(),
            width: 1,
            height: 2,
            shape: vec![], // Will be populated below
            material: MaterialType::Steel,
            item_type: ItemType::Weapon,
            rarity: ItemRarity::Common,
            price: 5,
            tags: vec![ItemTag::Weapon],
            synergies: vec![],
            attack: 10.0,
            defense: 0.0,
            speed: 0.0,
        },
        ItemDefinition {
            id: "hero_sword".to_string(),
            name: "Hero Sword".to_string(),
            width: 1, height: 2, shape: vec![],
            material: MaterialType::Steel,
            item_type: ItemType::Weapon,
            rarity: ItemRarity::Rare,
            price: 15,
            tags: vec![ItemTag::Weapon],
            synergies: vec![],
            attack: 18.0, defense: 0.0, speed: 10.0,
        },
        ItemDefinition {
            id: "hero_longsword".to_string(),
            name: "Hero Longsword".to_string(),
            width: 1, height: 3, shape: vec![],
            material: MaterialType::Steel,
            item_type: ItemType::Weapon,
            rarity: ItemRarity::Epic,
            price: 30,
            tags: vec![ItemTag::Weapon],
            synergies: vec![],
            attack: 30.0, defense: 0.0, speed: 15.0,
        },
        ItemDefinition {
            id: "silver_dagger".to_string(),
            name: "Silver Dagger".to_string(),
            width: 1,
            height: 1,
            shape: vec![],
            material: MaterialType::Silver,
            item_type: ItemType::Weapon,
            rarity: ItemRarity::Rare,
            price: 7,
            tags: vec![ItemTag::Weapon],
            synergies: vec![],
            attack: 8.0,
            defense: 0.0,
            speed: 5.0,
        },
        ItemDefinition {
            id: "health_potion".to_string(),
            name: "Health Potion".to_string(),
            width: 1,
            height: 1,
            shape: vec![],
            material: MaterialType::Flesh,
            item_type: ItemType::Consumable,
            rarity: ItemRarity::Common,
            price: 3,
            tags: vec![ItemTag::Potion],
            synergies: vec![],
            attack: 0.0,
            defense: 0.0,
            speed: 0.0,
        },
        ItemDefinition {
            id: "whetstone".to_string(),
            name: "Whetstone".to_string(),
            width: 1,
            height: 1,
            shape: vec![],
            material: MaterialType::Steel,
            item_type: ItemType::Consumable,
            rarity: ItemRarity::Common,
            price: 4,
            tags: vec![ItemTag::Valuable],
            synergies: vec![
                SynergyDefinition {
                    offset: IVec2::new(1, 0),
                    target_tags: vec![ItemTag::Weapon],
                    effect: SynergyEffect::BuffTarget { stat: StatType::Attack, value: 5.0 }
                },
                SynergyDefinition {
                    offset: IVec2::new(-1, 0),
                    target_tags: vec![ItemTag::Weapon],
                    effect: SynergyEffect::BuffTarget { stat: StatType::Attack, value: 5.0 }
                },
                SynergyDefinition {
                    offset: IVec2::new(0, 1),
                    target_tags: vec![ItemTag::Weapon],
                    effect: SynergyEffect::BuffTarget { stat: StatType::Attack, value: 5.0 }
                },
                SynergyDefinition {
                    offset: IVec2::new(0, -1),
                    target_tags: vec![ItemTag::Weapon],
                    effect: SynergyEffect::BuffTarget { stat: StatType::Attack, value: 5.0 }
                }
            ],
            attack: 0.0,
            defense: 0.0,
            speed: 0.0,
        },
        // Armor and Catalyst
        ItemDefinition {
            id: "leather_armor".to_string(),
            name: "Leather Armor".to_string(),
            width: 2, height: 2, shape: vec![],
            material: MaterialType::Fabric,
            item_type: ItemType::Armor,
            rarity: ItemRarity::Common,
            price: 8,
            tags: vec![ItemTag::Armor],
            synergies: vec![],
            attack: 0.0, defense: 25.0, speed: 0.0,
        },
        ItemDefinition {
            id: "blood_amulet".to_string(),
            name: "Blood Amulet".to_string(),
            width: 1, height: 1, shape: vec![],
            material: MaterialType::Crystal,
            item_type: ItemType::Accessory,
            rarity: ItemRarity::Rare,
            price: 15,
            tags: vec![ItemTag::Valuable, ItemTag::Accessory],
            synergies: vec![],
            attack: 0.0, defense: 0.0, speed: 0.0,
        },
        ItemDefinition {
            id: "vampiric_armor".to_string(),
            name: "Vampiric Armor".to_string(),
            width: 2, height: 2, shape: vec![],
            material: MaterialType::Flesh,
            item_type: ItemType::Armor,
            rarity: ItemRarity::Epic,
            price: 40,
            tags: vec![ItemTag::Armor],
            synergies: vec![],
            attack: 0.0, defense: 40.0, speed: 0.0,
        },
        // Adding more items to test rarity
        ItemDefinition {
            id: "epic_shield".to_string(),
            name: "Epic Shield".to_string(),
            width: 2,
            height: 2,
            shape: vec![],
            material: MaterialType::Steel,
            item_type: ItemType::Weapon,
            rarity: ItemRarity::Epic,
            price: 12,
            tags: vec![ItemTag::Weapon],
            synergies: vec![],
            attack: 2.0,
            defense: 20.0,
            speed: -2.0,
        },
        ItemDefinition {
            id: "legendary_bow".to_string(),
            name: "Legendary Bow".to_string(),
            width: 1,
            height: 3,
            shape: vec![],
            material: MaterialType::Flesh,
            item_type: ItemType::Weapon,
            rarity: ItemRarity::Legendary,
            price: 25,
            tags: vec![ItemTag::Weapon],
            synergies: vec![],
            attack: 15.0,
            defense: 0.0,
            speed: 10.0,
        },
        ItemDefinition {
             id: "unique_charm".to_string(),
             name: "Unique Charm".to_string(),
             width: 1,
             height: 1,
             shape: vec![],
             material: MaterialType::Silver,
             item_type: ItemType::Consumable,
             rarity: ItemRarity::Unique,
             price: 50,
             tags: vec![ItemTag::Valuable],
             synergies: vec![],
             attack: 0.0,
             defense: 0.0,
             speed: 0.0,
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

    // Populate Recipes
    item_db.recipes = vec![
        RecipeDefinition {
            result_item_id: "hero_sword".to_string(),
            ingredients: vec![
                RecipeIngredient { item_id: "wooden_sword".to_string(), is_catalyst: false },
                RecipeIngredient { item_id: "whetstone".to_string(), is_catalyst: false },
            ],
        },
        RecipeDefinition {
            result_item_id: "hero_longsword".to_string(),
            ingredients: vec![
                RecipeIngredient { item_id: "hero_sword".to_string(), is_catalyst: false },
                RecipeIngredient { item_id: "whetstone".to_string(), is_catalyst: false },
                RecipeIngredient { item_id: "whetstone".to_string(), is_catalyst: false },
            ],
        },
        RecipeDefinition {
            result_item_id: "vampiric_armor".to_string(),
            ingredients: vec![
                RecipeIngredient { item_id: "leather_armor".to_string(), is_catalyst: false },
                RecipeIngredient { item_id: "blood_amulet".to_string(), is_catalyst: true },
            ],
        },
    ];

    info!("ItemDatabase loaded with {} items and {} recipes.", item_db.items.len(), item_db.recipes.len());
}
