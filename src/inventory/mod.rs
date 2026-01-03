use bevy::prelude::*;
use bevy::utils::HashMap;

pub struct GridInventoryPlugin;

impl Plugin for GridInventoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InventoryGridState>()
           .register_type::<InventorySlot>()
           .register_type::<Item>()
           .register_type::<GridPosition>()
           .register_type::<ItemSize>();
    }
}

#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
pub struct InventoryGridState {
   pub cells: HashMap<IVec2, Entity>,
   pub width: i32,
   pub height: i32,
}

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct InventorySlot {
    pub position: IVec2,
}

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct Item;

#[derive(Component, Reflect, Default, Clone, Copy)]
#[reflect(Component)]
pub struct GridPosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Component, Reflect, Default, Clone, Copy)]
#[reflect(Component)]
pub struct ItemSize {
    pub width: i32,
    pub height: i32,
}
