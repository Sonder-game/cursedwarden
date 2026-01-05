use bevy::prelude::*;
use bevy::utils::HashMap;
use crate::plugins::core::GameState; // Убедитесь, что GameState доступен
use crate::plugins::items::{ItemDefinition, ItemType}; // Предполагается наличие определений

pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
   fn build(&self, app: &mut App) {
       app
           // Ресурсы
          .init_resource::<InventoryGridState>()
          .init_resource::<DragState>()
           // События
          .add_event::<InventoryChangedEvent>()
           // Системы жизненного цикла
          .add_systems(OnEnter(GameState::EveningPhase), setup_inventory_ui)
          .add_systems(OnExit(GameState::EveningPhase), cleanup_inventory)
           // Системы обновления (работают только в фазе инвентаря)
          .add_systems(
               Update,
               (
                   update_grid_visuals, // Синхронизация GridPosition -> UI Style
                   handle_keyboard_rotation, // Вращение на клавишу R
                   debug_grid_gizmos, // Отрисовка границ (для отладки)
               ).run_if(in_state(GameState::EveningPhase))
           )
           // Регистрация Наблюдателей (Observers) для Drag & Drop
          .add_observer(on_drag_start)
          .add_observer(on_drag)
          .add_observer(on_drag_end);
   }
}

// ============================================================================
// КОМПОНЕНТЫ
// ============================================================================

/// Основной компонент предмета инвентаря
#[derive(Component)]
pub struct InventoryItem {
   pub item_id: String,
   /// Базовая форма (список смещений от 0,0)
   pub shape: Vec<IVec2>,
   pub width: i32,
   pub height: i32,
}

/// Компонент сумки. Сумка - это тоже InventoryItem, но она ГЕНЕРИРУЕТ слоты.
#[derive(Component)]
pub struct Bag {
   /// Форма предоставляемой области (какие слоты создает сумка)
   pub provided_slots: Vec<IVec2>,
}

/// Логическая позиция в сетке (координаты левого верхнего угла - Anchor Point)
#[derive(Component)]
pub struct GridPosition(pub IVec2);

/// Текущий поворот (0=0, 1=90, 2=180, 3=270 градусов)
#[derive(Component)]
pub struct ItemRotation(pub u8);

/// Маркер для корневого узла UI инвентаря
#[derive(Component)]
pub struct InventoryRoot;

// ============================================================================
// РЕСУРСЫ И СОСТОЯНИЕ
// ============================================================================

/// Глобальное состояние сетки. "Источник правды" для коллизий.
#[derive(Resource, Default)]
pub struct InventoryGridState {
   /// Карта всех валидных слотов.
   /// Ключ: Координата (x,y). Значение: Данные о слоте.
   pub slots: HashMap<IVec2, SlotData>,
   /// Кэшированные границы сетки (для камеры и UI)
   pub bounds: IRect,
}

#[derive(Clone)]
pub struct SlotData {
   /// Какая сумка создала этот слот
   pub bag_entity: Entity,
   /// Какой предмет занимает этот слот (если есть)
   pub occupier: Option<Entity>,
}

/// Состояние текущего перетаскивания
#[derive(Resource, Default)]
pub struct DragState {
   /// Исходная позиция (для возврата при неудаче)
   pub original_pos: Option<IVec2>,
   pub original_rotation: Option<u8>,
   /// Список предметов внутри сумки (если тащим сумку), чтобы двигать их вместе
   pub attached_items: Vec<Entity>,
   /// Смещение курсора относительно левого верхнего угла предмета
   pub drag_offset: Vec2,
}

/// Событие, которое рассылается при успешном изменении инвентаря
#[derive(Event)]
pub struct InventoryChangedEvent;

// ============================================================================
// КОНСТАНТЫ
// ============================================================================

const SLOT_SIZE: f32 = 64.0;
const SLOT_GAP: f32 = 2.0;
const TOTAL_CELL_SIZE: f32 = SLOT_SIZE + SLOT_GAP;

// ============================================================================
// ЛОГИКА СЕТКИ (GRID LOGIC)
// ============================================================================

impl InventoryGridState {
   /// Полная перестройка состояния сетки.
   /// Должна вызываться после любого изменения позиций или поворотов.
   pub fn rebuild(
       &mut self,
       q_bags: &Query<(Entity, &GridPosition, &ItemRotation, &Bag)>,
       q_items: &Query<(Entity, &GridPosition, &ItemRotation, &InventoryItem), Without<Bag>>,
   ) {
       self.slots.clear();
       self.bounds = IRect::new(0, 0, 0, 0);

       // 1. Фаза "Раскатывания сумок": Создаем слоты
       for (bag_entity, bag_pos, bag_rot, bag) in q_bags.iter() {
           let shape = rotate_shape(&bag.provided_slots, bag_rot.0);
           for offset in shape {
               let slot_pos = bag_pos.0 + offset;

               // Если слоты сумок перекрываются, побеждает последняя (упрощение)
               // В идеале можно запретить перекрытие сумок при установке.
               self.slots.insert(slot_pos, SlotData {
                   bag_entity,
                   occupier: None,
               });

               // Расширяем границы
               self.bounds.max = self.bounds.max.max(slot_pos);
               self.bounds.min = self.bounds.min.min(slot_pos);
           }
       }

       // 2. Фаза "Заполнения": Размещаем предметы
       for (item_entity, item_pos, item_rot, item) in q_items.iter() {
           let shape = rotate_shape(&item.shape, item_rot.0);
           for offset in shape {
               let cell_pos = item_pos.0 + offset;

               if let Some(slot) = self.slots.get_mut(&cell_pos) {
                   // Если слот уже занят - это коллизия (которую мы должны были предотвратить при DragDrop)
                   // Но при загрузке сохранения или багах это возможно.
                   if slot.occupier.is_some() {
                       warn!("Double occupancy at {:?} by item {:?}", cell_pos, item_entity);
                   }
                   slot.occupier = Some(item_entity);
               } else {
                   // Предмет "висит в воздухе" (вне сумок).
                   warn!("Item {:?} at {:?} is floating (no bag)", item_entity, cell_pos);
               }
           }
       }
   }

   /// Проверяет валидность размещения предмета (обычного)
   pub fn can_place_item(
       &self,
       shape: &Vec<IVec2>,
       pos: IVec2,
       rot: u8,
       exclude_entity: Option<Entity>, // Игнорировать самого себя
   ) -> bool {
       let rotated_shape = rotate_shape(shape, rot);

       for offset in rotated_shape {
           let target_pos = pos + offset;

           match self.slots.get(&target_pos) {
               Some(slot) => {
                   // Слот существует (есть сумка). Проверяем занятость.
                   if let Some(occupier) = slot.occupier {
                       if Some(occupier) != exclude_entity {
                           return false; // Занято другим предметом
                       }
                   }
               },
               None => return false, // Нет слота (пустота)
           }
       }
       true
   }

   /// Проверяет валидность размещения сумки (сумки не должны перекрываться друг с другом)
   pub fn can_place_bag(
       &self,
       bag_shape: &Vec<IVec2>,
       pos: IVec2,
       rot: u8,
       exclude_bag: Option<Entity>,
   ) -> bool {
       let rotated_shape = rotate_shape(bag_shape, rot);

       for offset in rotated_shape {
           let target_pos = pos + offset;

           // Проверяем, есть ли уже слот в этой координате от ДРУГОЙ сумки
           if let Some(slot) = self.slots.get(&target_pos) {
               if Some(slot.bag_entity) != exclude_bag {
                   return false; // Перекрытие с чужой сумкой
               }
           }
       }
       true
   }

   /// Finds a free spot for an item definition
   pub fn find_free_spot(&self, def: &ItemDefinition) -> Option<IVec2> {
        // Iterate through valid slots bounds
        let min_x = self.bounds.min.x;
        let max_x = self.bounds.max.x;
        let min_y = self.bounds.min.y;
        let max_y = self.bounds.max.y;

        // Naive search: try every position in bounds
        // Optimization: Iterate through keys of slots?
        // But slots map is sparse. We need to check if *all* cells of item shape land on valid slots.

        // Let's iterate over known slots as potential anchor points.
        // Or just iterate bounds if they are small.
        // Bounds can be large if bags are far apart.
        // But let's try strict bounds iteration.
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let pos = IVec2::new(x, y);
                if self.can_place_item(&def.shape, pos, 0, None) {
                    return Some(pos);
                }
            }
        }
        None
   }
}

/// Поворот вектора формы на 90 градусов N раз
fn rotate_shape(shape: &Vec<IVec2>, rot: u8) -> Vec<IVec2> {
   let steps = rot % 4;
   if steps == 0 { return shape.clone(); }

   shape.iter().map(|p| {
       let mut v = *p;
       for _ in 0..steps {
           // Формула поворота вектора на 90 град по часовой: (x, y) -> (-y, x)
           // В экранных координатах (Y вниз) это: (x, y) -> (-y, x)
           v = IVec2::new(-v.y, v.x);
       }
       v
   }).collect()
}

// ============================================================================
// СИСТЕМЫ BEVY PICKING (DRAG & DROP)
// ============================================================================

fn on_drag_start(
   trigger: Trigger<Pointer<DragStart>>,
   mut commands: Commands,
   q_items: Query<(Entity, &GridPosition, &ItemRotation, Option<&Bag>), With<InventoryItem>>,
   mut drag_state: ResMut<DragState>,
   mut q_node: Query<(&mut ZIndex, &Node)>,
) {
   let entity = trigger.entity();

   if let Ok((_e, grid_pos, rot, is_bag)) = q_items.get(entity) {
       // 1. Сохраняем состояние
       drag_state.original_pos = Some(grid_pos.0);
       drag_state.original_rotation = Some(rot.0);
       drag_state.attached_items.clear();

       // 2. Логика "Сумка тащит вещи"
       if is_bag.is_some() {
           // Находим все предметы, чей якорь совпадает с позицией сумки (упрощение)
           // В реальном проекте тут нужна проверка "содержится ли предмет в слотах этой сумки"
           // Для этого можно использовать InventoryGridState, но он сейчас "старый".
           // Поэтому пройдемся по всем предметам.
           // (Для прототипа: если GridPosition предмета совпадает с GridPosition сумки - тащим).
           // Более надежно: если предмет лежит в слоте, который принадлежит этой сумке.

           // Мы это реализуем в on_drop, здесь просто помечаем факт драга сумки.
       }

       // 3. Визуальный отклик
       if let Ok((mut z_index, _node)) = q_node.get_mut(entity) {
           *z_index = ZIndex(100); // Поднять над остальными

           // Вычисляем смещение хвата, чтобы предмет не "прыгал" к курсору
           // Но Pointer<DragStart> не дает точную позицию относительно ноды.
           // Упростим: просто будем двигать через дельту в on_drag.
       }

       // 4. CRITICAL FIX: Игнорируем Picking для перетаскиваемого предмета.
       // Это позволяет лучу мыши проходить СКВОЗЬ предмет и видеть слоты под ним.
       commands.entity(entity).insert(PickingBehavior::IGNORE);
   }
}

fn on_drag(
   trigger: Trigger<Pointer<Drag>>,
   mut q_node: Query<&mut Node>,
) {
   let entity = trigger.entity();
   let drag_event = trigger.event();

   if let Ok(mut node) = q_node.get_mut(entity) {
       // Прямое изменение позиции UI (визуальное, не логическое)
       if let Val::Px(left) = node.left {
           node.left = Val::Px(left + drag_event.delta.x);
       }
       if let Val::Px(top) = node.top {
           node.top = Val::Px(top + drag_event.delta.y);
       }
   }
}

fn on_drag_end(
   trigger: Trigger<Pointer<DragEnd>>,
   mut commands: Commands,
   mut q_items: Query<(Entity, &mut GridPosition, &mut ItemRotation, &InventoryItem, &mut Node, Option<&Bag>)>,

   // Нам нужны queries для rebuild
   q_bags_ro: Query<(Entity, &GridPosition, &ItemRotation, &Bag)>,
   q_items_ro: Query<(Entity, &GridPosition, &ItemRotation, &InventoryItem), Without<Bag>>,

   mut grid_state: ResMut<InventoryGridState>,
   mut drag_state: ResMut<DragState>,
   mut ev_changed: EventWriter<InventoryChangedEvent>,
) {
   let entity = trigger.entity();

   // 1. Возвращаем интерактивность (снимаем IGNORE)
   commands.entity(entity).remove::<PickingBehavior>();

   let mut bag_move_data: Option<(Entity, IVec2)> = None;

   if let Ok((_e, mut grid_pos, mut rot, item_def, node, is_bag)) = q_items.get_mut(entity) {

       // 2. Рассчитываем целевой слот (Snap to Grid)
       // Предполагаем, что контейнер начинается в 0,0 родителя.
       let current_left = if let Val::Px(v) = node.left { v } else { 0.0 };
       let current_top = if let Val::Px(v) = node.top { v } else { 0.0 };

       let target_x = (current_left / TOTAL_CELL_SIZE).round() as i32;
       let target_y = (current_top / TOTAL_CELL_SIZE).round() as i32;
       let target_pos = IVec2::new(target_x, target_y);

       // 3. Валидация
       let mut valid = false;

       if is_bag.is_some() {
           // ЛОГИКА СУМКИ
           if let Ok((_, _, _, bag)) = q_bags_ro.get(entity) {
               if grid_state.can_place_bag(&bag_def_to_shape(&bag.provided_slots), target_pos, rot.0, Some(entity)) {
                   valid = true;
               }
           }
       } else {
           // ЛОГИКА ПРЕДМЕТА
           if grid_state.can_place_item(&item_def.shape, target_pos, rot.0, Some(entity)) {
               valid = true;
           }
       }

       // 4. Применение результатов
       if valid {
           // Успех!

           if is_bag.is_some() {
               let delta = target_pos - drag_state.original_pos.unwrap_or(target_pos);
               if delta != IVec2::ZERO {
                   // Defer execution to avoid double borrow
                   bag_move_data = Some((entity, delta));
               }
           }

           grid_pos.0 = target_pos;
           ev_changed.send(InventoryChangedEvent);
       } else {
           // Провал! Откат.
           if let Some(orig) = drag_state.original_pos {
               grid_pos.0 = orig;
           }
           if let Some(orig_rot) = drag_state.original_rotation {
               rot.0 = orig_rot;
           }
       }
   }

   // Apply deferred bag move
   if let Some((bag_entity, delta)) = bag_move_data {
       move_bag_contents(bag_entity, delta, &mut q_items, &grid_state);
   }

   // 5. Перестраиваем глобальную сетку
   grid_state.rebuild(&q_bags_ro, &q_items_ro);
}

/// Хелпер для перемещения содержимого сумки
fn move_bag_contents(
   bag_entity: Entity,
   delta: IVec2,
   q_items_mut: &mut Query<(Entity, &mut GridPosition, &mut ItemRotation, &InventoryItem, &mut Node, Option<&Bag>)>,
   grid_state: &InventoryGridState,
) {
   // В ECS "грязно" итерировать и мутировать одновременно.
   // Нам нужно найти ID предметов, которые "принадлежат" сумке.
   // Используем grid_state до его обновления (он хранит старое состояние).

   let mut items_to_move = Vec::new();

   for (_pos, slot) in &grid_state.slots {
       if slot.bag_entity == bag_entity {
           if let Some(item_ent) = slot.occupier {
               if !items_to_move.contains(&item_ent) {
                   items_to_move.push(item_ent);
               }
           }
       }
   }

   // Применяем дельту
   for item_e in items_to_move {
       // Нужно получить мутабельный доступ. Это сложно в одном запросе с q_items_mut.
       // Rust не даст дважды заимствовать.
       // Поэтому в on_drag_end мы используем q_items (который включает и сумки и предметы).
       // Мы используем `get_mut` по ID.
       if let Ok((_, mut pos, _, _, _, _)) = q_items_mut.get_mut(item_e) {
           pos.0 += delta;
       }
   }
}

// Вспомогательный конвертер (так как Bag хранит provided_slots как форму)
fn bag_def_to_shape(vec: &Vec<IVec2>) -> Vec<IVec2> {
   vec.clone()
}

// ============================================================================
// СИСТЕМЫ ОБНОВЛЕНИЯ И ВИЗУАЛИЗАЦИИ
// ============================================================================

/// Синхронизирует логические координаты GridPosition с визуальными Node.left/top
fn update_grid_visuals(
   mut q_items: Query<(Entity, &GridPosition, &mut Node, &mut ZIndex, Option<&PickingBehavior>), (With<InventoryItem>, Changed<GridPosition>)>,
) {
   for (_entity, pos, mut node, mut z_index, picking) in q_items.iter_mut() {
       // Если предмет сейчас перетаскивается (имеет IGNORE), мы не должны жестко сетить его позицию,
       // иначе он будет "мерцать", пытаясь вернуться в слот.
       // Проверяем PickingBehavior.
       if let Some(p) = picking {
           if *p == PickingBehavior::IGNORE {
               continue;
           }
       }

       // Плавная интерполяция была бы лучше, но для начала жесткая привязка
       node.left = Val::Px(pos.0.x as f32 * TOTAL_CELL_SIZE);
       node.top = Val::Px(pos.0.y as f32 * TOTAL_CELL_SIZE);

       // Сброс Z-Index
       *z_index = ZIndex(10);
   }
}

fn handle_keyboard_rotation(
   input: Res<ButtonInput<KeyCode>>,
   mut q_items: Query<(&mut ItemRotation, &mut Node, &InventoryItem, &PickingBehavior)>,
) {
   if input.just_pressed(KeyCode::KeyR) {
       for (mut rot, mut node, _item, picking) in q_items.iter_mut() {
           // Only rotate if currently being dragged (PickingBehavior::IGNORE)
           if *picking != PickingBehavior::IGNORE {
               continue;
           }

           // Вращаем
           rot.0 = (rot.0 + 1) % 4;

           // ВАЖНО: Нужно визуально повернуть предмет (свапнуть ширину и высоту ноды)
           let temp = node.width;
           node.width = node.height;
           node.height = temp;
       }
   }
}

// ============================================================================
// ИНИЦИАЛИЗАЦИЯ (SETUP)
// ============================================================================

fn setup_inventory_ui(mut commands: Commands) {
   // 1. Спавним контейнер
   commands.spawn((
       Node {
           width: Val::Percent(100.0),
           height: Val::Percent(100.0),
           // Центрируем сетку
           justify_content: JustifyContent::Center,
           align_items: AlignItems::Center,
          ..default()
       },
       InventoryRoot,
   )).with_children(|parent| {
       // 2. Сама область сетки (относительная база для абсолютных предметов)
       parent.spawn(Node {
           width: Val::Px(800.0), // Достаточно места
           height: Val::Px(600.0),
           position_type: PositionType::Relative,
           border: UiRect::all(Val::Px(2.0)),
          ..default()
       }).with_children(|grid_area| {
           // Здесь будут жить наши предметы и сумки.
           // Спавним стартовую сумку
           spawn_test_bag(grid_area, IVec2::new(2, 2));
           spawn_test_item(grid_area, IVec2::new(2, 2)); // Внутри сумки
       });
   });
}

// Тестовые спавнеры (в реальности это делает ItemFactory)
pub fn spawn_test_bag(parent: &mut ChildBuilder, pos: IVec2) {
   // Сумка 2x2
   let shape = vec![IVec2::new(0,0), IVec2::new(1,0), IVec2::new(0,1), IVec2::new(1,1)];

   parent.spawn((
       InventoryItem { item_id: "bag_starter".into(), width: 2, height: 2, shape: shape.clone() },
       Bag { provided_slots: shape },
       GridPosition(pos),
       ItemRotation(0),
       Node {
           position_type: PositionType::Absolute,
           width: Val::Px(2.0 * TOTAL_CELL_SIZE - SLOT_GAP),
           height: Val::Px(2.0 * TOTAL_CELL_SIZE - SLOT_GAP),
           left: Val::Px(pos.x as f32 * TOTAL_CELL_SIZE),
           top: Val::Px(pos.y as f32 * TOTAL_CELL_SIZE),
          ..default()
       },
       BackgroundColor(Color::srgb(0.5, 0.3, 0.1)), // Коричневый
       PickingBehavior::default(), // Включаем взаимодействие
   ));
}

pub fn spawn_test_item(parent: &mut ChildBuilder, pos: IVec2) {
   // Меч 1x2
   let shape = vec![IVec2::new(0,0), IVec2::new(0,1)];

   parent.spawn((
       InventoryItem { item_id: "sword".into(), width: 1, height: 2, shape },
       GridPosition(pos),
       ItemRotation(0),
       Node {
           position_type: PositionType::Absolute,
           width: Val::Px(1.0 * TOTAL_CELL_SIZE - SLOT_GAP),
           height: Val::Px(2.0 * TOTAL_CELL_SIZE - SLOT_GAP),
           left: Val::Px(pos.x as f32 * TOTAL_CELL_SIZE),
           top: Val::Px(pos.y as f32 * TOTAL_CELL_SIZE),
          ..default()
       },
       BackgroundColor(Color::srgb(0.8, 0.8, 0.9)), // Сталь
       PickingBehavior::default(),
   ));
}

fn cleanup_inventory(mut commands: Commands, q: Query<Entity, With<InventoryRoot>>) {
   for e in q.iter() {
       commands.entity(e).despawn_recursive();
   }
}

// Система отладки: рисует гизмо поверх слотов
fn debug_grid_gizmos(
   _gizmos: Gizmos,
   _grid_state: Res<InventoryGridState>,
) {
   // Для отладки координат (можно отключить в релизе)
   // Gizmos рисуются в WorldSpace, а UI в ScreenSpace.
   // Без спец. камеры они не совпадут. Оставляем пустым или используем UI-бордеры.
}
