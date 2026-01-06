use bevy::prelude::*;
use bevy::utils::HashMap;
use crate::plugins::core::GameState;
use crate::plugins::items::{ItemDefinition, ItemType};

/// Плагин, управляющий всей логикой инвентаря, сетки и взаимодействия.
pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
   fn build(&self, app: &mut App) {
       app
           // Ресурсы: Единый источник правды о топологии сетки
          .init_resource::<InventoryGridState>()
          .init_resource::<DragState>()
           // События: Сигнализируют об изменениях для пересчета статов
          .add_event::<InventoryChangedEvent>()
           // Системы жизненного цикла UI
          .add_systems(OnEnter(GameState::EveningPhase), setup_inventory_ui)
          .add_systems(OnExit(GameState::EveningPhase), cleanup_inventory)
           // Системы обновления (работают только в фазе инвентаря)
          .add_systems(
               Update,
               (
                   update_grid_visuals, // Синхронизация логики ECS -> UI
                   handle_keyboard_rotation, // Вращение на клавишу R
                   debug_grid_gizmos, // Визуальная отладка (заглушка)
               ).run_if(in_state(GameState::EveningPhase))
           )
           // Bevy Picking Observers: Новая система событий для Drag & Drop (Bevy 0.15)
          .add_observer(on_drag_start)
          .add_observer(on_drag)
          .add_observer(on_drag_end);
   }
}

// ============================================================================
// КОМПОНЕНТЫ (COMPONENTS)
// ============================================================================

/// Основной компонент предмета. Хранит его ID и форму.
#[derive(Component)]
pub struct InventoryItem {
   pub item_id: String,
   /// Список относительных координат, которые занимает предмет.
   /// (0,0) - это верхний левый угол (Anchor/Якорь).
   pub shape: Vec<IVec2>,
}

/// Компонент сумки. Сумка - это тоже предмет, но она СОЗДАЕТ (проецирует) слоты.
#[derive(Component)]
pub struct Bag {
   /// Форма предоставляемых слотов (относительно Anchor).
   pub provided_slots: Vec<IVec2>,
}

/// Логическая позиция в сетке. Единый источник правды для логики.
/// IVec2(x, y). Ось Y растет вниз.
#[derive(Component)]
pub struct GridPosition(pub IVec2);

/// Текущий поворот: 0=0°, 1=90°, 2=180°, 3=270°.
#[derive(Component)]
pub struct ItemRotation(pub u8);

/// Маркер, определяющий, находится ли предмет в зоне "Хранилища" (Limbo).
#[derive(Component)]
pub struct InStorage;

// Маркеры для UI узлов
#[derive(Component)] struct InventoryRoot;
#[derive(Component)] pub struct InventoryGridContainer; // Зона активного инвентаря
#[derive(Component)] pub struct StorageContainer; // Зона "Limbo"

// ============================================================================
// РЕСУРСЫ (RESOURCES)
// ============================================================================

/// Глобальное состояние сетки. Используется для быстрых проверок коллизий (O(1)).
#[derive(Resource, Default)]
pub struct InventoryGridState {
   /// Карта слотов. Ключ - координата. Значение - данные о слоте.
   pub slots: HashMap<IVec2, SlotData>,
   /// Границы активной зоны (для ограничения движения сумок).
   pub bounds: IRect,
}

#[derive(Clone)]
pub struct SlotData {
   /// ID сущности сумки, которая создала этот слот.
   pub bag_entity: Entity,
   /// ID сущности предмета, который занимает этот слот (или None).
   pub occupier: Option<Entity>,
}

/// Состояние текущего перетаскивания.
#[derive(Resource, Default)]
pub struct DragState {
   /// Исходная позиция (для отката при невалидном сбросе).
   pub original_pos: Option<IVec2>,
   pub original_rotation: Option<u8>,
   pub was_in_storage: bool,
   /// Если тащим сумку, здесь хранятся ID предметов внутри неё для кинематики.
   pub attached_items: Vec<Entity>,
}

#[derive(Event)]
pub struct InventoryChangedEvent;

// ============================================================================
// КОНСТАНТЫ (Настройка визуального стиля)
// ============================================================================

const SLOT_SIZE: f32 = 64.0;
const SLOT_GAP: f32 = 2.0;
const TOTAL_CELL_SIZE: f32 = SLOT_SIZE + SLOT_GAP;

// ============================================================================
// ЛОГИКА СЕТКИ (GRID ALGORITHMS)
// ============================================================================

impl InventoryGridState {
   /// Публичный хелпер для доступа к логике вращения из других модулей (например, для отрисовки синергий).
   pub fn get_rotated_shape(shape: &Vec<IVec2>, rot: u8) -> Vec<IVec2> {
       rotate_shape(shape, rot)
   }

   /// Полная перестройка карты слотов. Вызывается после любого изменения.
   /// Это гарантирует целостность данных и решает проблему рассинхронизации.
   pub fn rebuild(
       &mut self,
       q_bags: &Query<(Entity, &GridPosition, &ItemRotation, &Bag), Without<InStorage>>,
       q_items: &Query<(Entity, &GridPosition, &ItemRotation, &InventoryItem), (Without<Bag>, Without<InStorage>)>,
   ) {
       self.slots.clear();
       self.bounds = IRect::new(0, 0, 0, 0);

       // 1. Проецируем все сумки на сетку (создаем валидные слоты)
       for (bag_entity, bag_pos, bag_rot, bag) in q_bags.iter() {
           let shape = rotate_shape(&bag.provided_slots, bag_rot.0);
           for offset in shape {
               let slot_pos = bag_pos.0 + offset;
               // Если сумки перекрываются, последняя "побеждает".
               // В идеале можно добавить логику запрета перекрытия сумок.
               self.slots.insert(slot_pos, SlotData {
                   bag_entity,
                   occupier: None,
               });
               // Расширяем границы сетки
               self.bounds.max = self.bounds.max.max(slot_pos);
               self.bounds.min = self.bounds.min.min(slot_pos);
           }
       }

       // 2. Размещаем предметы в слотах
       for (item_entity, item_pos, item_rot, item) in q_items.iter() {
           let shape = rotate_shape(&item.shape, item_rot.0);
           for offset in shape {
               let cell_pos = item_pos.0 + offset;
               if let Some(slot) = self.slots.get_mut(&cell_pos) {
                   if slot.occupier.is_some() {
                       warn!("Коллизия! Клетка {:?} уже занята предметом {:?}.", cell_pos, slot.occupier);
                   }
                   slot.occupier = Some(item_entity);
               } else {
                   // Предмет висит в воздухе (вне сумки).
                   // В момент rebuild это считается невалидным состоянием, но мы не удаляем предмет,
                   // чтобы избежать потери данных при багах.
               }
           }
       }
   }

   /// Проверяет, можно ли разместить ПРЕДМЕТ в заданных координатах.
   pub fn can_place_item(
       &self,
       shape: &Vec<IVec2>,
       pos: IVec2,
       rot: u8,
       exclude_entity: Option<Entity>,
       target_is_storage: bool,
   ) -> bool {
       // В хранилище (Storage) всегда можно класть (упрощение: бесконечная емкость).
       if target_is_storage {
           return true;
       }

       let rotated_shape = rotate_shape(shape, rot);
       for offset in rotated_shape {
           let target_pos = pos + offset;
           match self.slots.get(&target_pos) {
               Some(slot) => {
                   // Слот существует (лежит на сумке). Проверяем занятость.
                   if let Some(occupier) = slot.occupier {
                       // Если занято не нами самими - это коллизия.
                       if Some(occupier)!= exclude_entity {
                           return false;
                       }
                   }
               },
               None => return false, // Нет сумки под предметом -> Нельзя положить.
           }
       }
       true
   }

   /// Проверяет, можно ли разместить СУМКУ. Сумки не должны перекрывать друг друга.
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
           if let Some(slot) = self.slots.get(&target_pos) {
               if Some(slot.bag_entity)!= exclude_bag {
                   return false; // Наехали на другую сумку
               }
           }
       }
       true
   }

   // Хелпер для ИИ/Магазина: Найти первое свободное место
   pub fn find_free_spot(&self, def: &ItemDefinition) -> Option<IVec2> {
       let min = self.bounds.min;
       let max = self.bounds.max;
       for y in min.y..=max.y {
           for x in min.x..=max.x {
               let pos = IVec2::new(x, y);
               // Пробуем с нулевым поворотом
               if self.can_place_item(&def.shape, pos, 0, None, false) {
                   return Some(pos);
               }
           }
       }
       None
   }
}

/// Математика вращения векторов на дискретной сетке (90 град CW)
fn rotate_shape(shape: &Vec<IVec2>, rot: u8) -> Vec<IVec2> {
   let steps = rot % 4;
   if steps == 0 { return shape.clone(); }
   shape.iter().map(|p| {
       let mut v = *p;
       for _ in 0..steps {
           // Формула поворота на 90 град по часовой стрелке в экранных координатах (Y вниз):
           // (x, y) -> (-y, x)
           v = IVec2::new(-v.y, v.x);
       }
       v
   }).collect()
}

// Хелпер для вычисления Bounding Box в пикселях (для спавна)
fn calculate_bounding_box(shape: &Vec<IVec2>, rotation_step: u8) -> (i32, i32, i32, i32) {
   let rotated_shape = rotate_shape(shape, rotation_step);
   if rotated_shape.is_empty() { return (0, 0, 1, 1); }
   let min_x = rotated_shape.iter().map(|v| v.x).min().unwrap();
   let max_x = rotated_shape.iter().map(|v| v.x).max().unwrap();
   let min_y = rotated_shape.iter().map(|v| v.y).min().unwrap();
   let max_y = rotated_shape.iter().map(|v| v.y).max().unwrap();
   (min_x, min_y, max_x - min_x + 1, max_y - min_y + 1)
}

// ============================================================================
// СИСТЕМА ВЗАИМОДЕЙСТВИЯ (BEVY PICKING OBSERVERS)
// ============================================================================

/// Начало перетаскивания (ЛКМ нажат)
fn on_drag_start(
   trigger: Trigger<Pointer<DragStart>>,
   mut commands: Commands,
   q_items: Query<(Entity, &GridPosition, &ItemRotation, Option<&Bag>, Has<InStorage>), With<InventoryItem>>,
   mut drag_state: ResMut<DragState>,
   mut q_node: Query<(&mut ZIndex, &Node)>,
   grid_state: Res<InventoryGridState>,
) {
   let entity = trigger.entity();
   if let Ok((_e, grid_pos, rot, is_bag, in_storage)) = q_items.get(entity) {
       // 1. Сохраняем состояние для отката (Undo)
       drag_state.original_pos = Some(grid_pos.0);
       drag_state.original_rotation = Some(rot.0);
       drag_state.was_in_storage = in_storage;
       drag_state.attached_items.clear();

       // 2. ЛОГИКА КИНЕМАТИКИ СУМОК (Bag Dragging)
       if is_bag.is_some() &&!in_storage {
           // Ищем все предметы, которые лежат в слотах этой сумки
           for (_slot_pos, slot_data) in &grid_state.slots {
               if slot_data.bag_entity == entity {
                   if let Some(occupier) = slot_data.occupier {
                       if!drag_state.attached_items.contains(&occupier) {
                           drag_state.attached_items.push(occupier);
                       }
                   }
               }
           }
       }

       // 3. Визуальный фидбек: Поднимаем предмет над всем UI (GlobalZIndex 100)
       // Важно: Мы используем обычный ZIndex внутри контейнера, но для перекрытия всего
       // можно использовать GlobalZIndex, если родительский контейнер имеет низкий Z.
       // Здесь мы просто ставим высокий локальный ZIndex.
       if let Ok((mut z_index, _)) = q_node.get_mut(entity) {
           *z_index = ZIndex(100);
       }

       // 4. КРИТИЧЕСКИ ВАЖНО: Игнорируем Picking для самого предмета во время драга.
       // Это позволяет лучу мыши "пробивать" предмет насквозь и видеть сетку под ним.
       commands.entity(entity).insert(PickingBehavior::IGNORE);
   }
}

/// Процесс перетаскивания (движение мыши) - обновляем только визуал
fn on_drag(
   trigger: Trigger<Pointer<Drag>>,
   mut q_node: Query<&mut Node>,
) {
   let entity = trigger.entity();
   let drag_event = trigger.event();
   if let Ok(mut node) = q_node.get_mut(entity) {
       // Обновляем визуальные координаты (Style). Логические (GridPosition) не трогаем.
       if let Val::Px(left) = node.left {
           node.left = Val::Px(left + drag_event.delta.x);
       }
       if let Val::Px(top) = node.top {
           node.top = Val::Px(top + drag_event.delta.y);
       }
   }
}

/// Завершение перетаскивания (ЛКМ отпущен) - основная логика
fn on_drag_end(
   trigger: Trigger<Pointer<DragEnd>>,
   mut commands: Commands,
   // Используем ParamSet для разрешения конфликтов заимствования
   mut queries: ParamSet<(
       Query<(Entity, &mut GridPosition, &mut ItemRotation, &InventoryItem, &Node, Option<&Bag>, Has<InStorage>)>, // Mutable
       (
           Query<(Entity, &GridPosition, &ItemRotation, &Bag), Without<InStorage>>, // Bags Read-Only
           Query<(Entity, &GridPosition, &ItemRotation, &InventoryItem), (Without<Bag>, Without<InStorage>)> // Items Read-Only
       )
   )>,
   mut grid_state: ResMut<InventoryGridState>,
   drag_state: Res<DragState>,
   mut ev_changed: EventWriter<InventoryChangedEvent>,
) {
   let entity = trigger.entity();

   // 1. Возвращаем интерактивность предмету
   commands.entity(entity).insert(PickingBehavior::default());

   let mut success = false;
   let mut delta = IVec2::ZERO;

   // Scope для мутабельного доступа
   {
       let mut q_mutable = queries.p0();
       if let Ok((_, mut grid_pos, mut rot, item_def, node, is_bag, _)) = q_mutable.get_mut(entity) {
           // 2. Рассчет координат привязки (Snapping)
           // Преобразуем координаты UI Node в индексы сетки.
           let current_left = if let Val::Px(v) = node.left { v } else { 0.0 };
           let current_top = if let Val::Px(v) = node.top { v } else { 0.0 };

           // Определяем, находится ли мышь над зоной Хранилища (хардкод порога по Y)
           let is_storage_drop = current_top > 400.0;

           let target_x = (current_left / TOTAL_CELL_SIZE).round() as i32;
           let target_y = (current_top / TOTAL_CELL_SIZE).round() as i32;
           let target_pos = IVec2::new(target_x, target_y);

           // 3. Валидация
           let mut valid = false;
           if is_storage_drop {
               // Логика сброса в хранилище (всегда разрешено)
               commands.entity(entity).insert(InStorage);
               valid = true;
           } else {
               // Если были в хранилище, убираем компонент
               commands.entity(entity).remove::<InStorage>();

               if let Some(bag) = is_bag {
                   // Перемещение Сумки: Проверяем, не наезжает ли она на другие сумки
                   if grid_state.can_place_bag(&bag.provided_slots, target_pos, rot.0, Some(entity)) {
                       valid = true;
                   }
               } else {
                   // Перемещение Предмета: Проверяем, попадает ли он в слоты сумок
                   if grid_state.can_place_item(&item_def.shape, target_pos, rot.0, Some(entity), false) {
                       valid = true;
                   }
               }
           }

           // 4. Применение или Откат
           if valid {
               // УСПЕХ
               // Если мы двигали сумку, вычисляем дельту для вложенных предметов
               if is_bag.is_some() &&!is_storage_drop {
                   delta = target_pos - drag_state.original_pos.unwrap_or(target_pos);
               }
               grid_pos.0 = target_pos;
               ev_changed.send(InventoryChangedEvent);
               success = true;
           } else {
               // ОТКАТ
               if let Some(orig) = drag_state.original_pos {
                   grid_pos.0 = orig;
               }
               if let Some(orig_rot) = drag_state.original_rotation {
                   rot.0 = orig_rot;
               }
               // Возврат флага InStorage
               if drag_state.was_in_storage {
                   commands.entity(entity).insert(InStorage);
               } else {
                   commands.entity(entity).remove::<InStorage>();
               }
           }
       }
   }

   // Обработка "пассажиров" (предметов внутри сумки)
   if success && delta!= IVec2::ZERO {
       let mut q_mutable = queries.p0();
       for attached_entity in &drag_state.attached_items {
           if let Ok((_, mut item_pos, _, _, _, _, _)) = q_mutable.get_mut(*attached_entity) {
               item_pos.0 += delta;
           }
       }
   }

   // 5. Перестройка состояния сетки для следующего кадра
   let (q_bags, q_items) = queries.p1();
   grid_state.rebuild(&q_bags, &q_items);
}

// ============================================================================
// ВИЗУАЛЬНАЯ СИНХРОНИЗАЦИЯ
// ============================================================================

/// Синхронизирует позицию UI Node с логической GridPosition.
/// Работает каждый кадр, обеспечивая плавность и коррекцию после Drop.
fn update_grid_visuals(
   mut q_items: Query<(Entity, &GridPosition, &mut Node, &mut ZIndex, Option<&PickingBehavior>), (With<InventoryItem>, Changed<GridPosition>)>,
) {
   for (_entity, pos, mut node, mut z_index, picking) in q_items.iter_mut() {
       // Не трогаем позицию, если предмет прямо сейчас перетаскивается
       if let Some(behavior) = picking {
           if *behavior == PickingBehavior::IGNORE {
               continue;
           }
       }

       // Жесткая привязка к сетке (Snapping)
       node.left = Val::Px(pos.0.x as f32 * TOTAL_CELL_SIZE);
       node.top = Val::Px(pos.0.y as f32 * TOTAL_CELL_SIZE);

       // Сброс Z-Index на нормальный уровень
       *z_index = ZIndex(10);
   }
}

/// Обработка вращения клавишей R
fn handle_keyboard_rotation(
   input: Res<ButtonInput<KeyCode>>,
   mut q_items: Query<(&mut ItemRotation, &mut Node, &PickingBehavior)>,
) {
   if input.just_pressed(KeyCode::KeyR) {
       for (mut rot, mut node, behavior) in q_items.iter_mut() {
           // Вращаем только тот предмет, который сейчас тащим
           if *behavior == PickingBehavior::IGNORE {
               rot.0 = (rot.0 + 1) % 4;
               // Визуальный поворот: меняем ширину и высоту местами
               let temp = node.width;
               node.width = node.height;
               node.height = temp;
           }
       }
   }
}

// ============================================================================
// ИНИЦИАЛИЗАЦИЯ UI
// ============================================================================

fn setup_inventory_ui(mut commands: Commands) {
   // Корневой контейнер
   commands.spawn((
       Node {
           width: Val::Percent(100.0),
           height: Val::Percent(100.0),
           justify_content: JustifyContent::FlexStart, // Сверху вниз
           align_items: AlignItems::Center,
           flex_direction: FlexDirection::Column,
          ..default()
       },
       InventoryRoot,
   )).with_children(|parent| {
       // 1. Активная зона инвентаря (где сумки)
       parent.spawn((
           Node {
               width: Val::Px(800.0),
               height: Val::Px(400.0),
               position_type: PositionType::Relative,
               border: UiRect::all(Val::Px(2.0)),
               margin: UiRect::bottom(Val::Px(20.0)),
              ..default()
           },
           InventoryGridContainer,
           BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
       ));

       // 2. Зона Хранилища (Limbo / Storage)
       parent.spawn((
           Node {
               width: Val::Px(800.0),
               height: Val::Px(200.0),
               position_type: PositionType::Relative,
               border: UiRect::all(Val::Px(2.0)),
              ..default()
           },
           StorageContainer,
           BackgroundColor(Color::srgb(0.15, 0.15, 0.25)), // Чуть синее
       )).with_children(|p| {
           p.spawn((
               Text::new("STORAGE (LIMBO)"),
               TextFont { font_size: 20.0,..default() },
               TextColor(Color::WHITE),
               Node {
                   position_type: PositionType::Absolute,
                   top: Val::Px(5.0),
                   left: Val::Px(5.0),
                  ..default()
               },
           ));
       });
   });
}

fn cleanup_inventory(mut commands: Commands, q: Query<Entity, With<InventoryRoot>>) {
   for e in q.iter() {
       commands.entity(e).despawn_recursive();
   }
}

fn debug_grid_gizmos(_gizmos: Gizmos) {}

// Хелпер для спавна (используется при загрузке и в магазине)
pub fn spawn_item_entity(
   commands: &mut Commands,
   container: Entity,
   def: &ItemDefinition,
   pos: IVec2,
   rotation: u8,
   _grid_state: &mut InventoryGridState,
) {
   let (_min_x, _min_y, width_slots, height_slots) = calculate_bounding_box(&def.shape, rotation);
   let width_px = width_slots as f32 * 64.0;
   let height_px = height_slots as f32 * 64.0;

   let left = pos.x as f32 * 64.0;
   let top = pos.y as f32 * 64.0;

   let is_bag = matches!(def.item_type, ItemType::Bag {.. });
   let z_idx = if is_bag { ZIndex(1) } else { ZIndex(10) };
   let bg_color = if is_bag { Color::srgb(0.4, 0.2, 0.1) } else { Color::srgb(0.5, 0.5, 0.8) };

   let mut entity_cmds = commands.spawn((
       Node {
           width: Val::Px(width_px),
           height: Val::Px(height_px),
           position_type: PositionType::Absolute,
           left: Val::Px(left),
           top: Val::Px(top),
           border: UiRect::all(Val::Px(1.0)),
          ..default()
       },
       BackgroundColor(bg_color),
       InventoryItem {
           item_id: def.id.clone(),
           shape: def.shape.clone(),
       },
       GridPosition(pos),
       ItemRotation(rotation),
       z_idx,
       PickingBehavior::default(),
   ));

   if is_bag {
       entity_cmds.insert(Bag { provided_slots: def.shape.clone() });
   }

   // Текст названия
   entity_cmds.with_children(|parent| {
       parent.spawn((
           Text::new(&def.name),
           TextFont { font_size: 14.0,..default() },
           TextColor(Color::WHITE),
           Node {
               position_type: PositionType::Absolute,
               left: Val::Px(2.0),
               top: Val::Px(2.0),
              ..default()
           },
           PickingBehavior::IGNORE, // Текст не должен блокировать драг
       ));
   });

   let entity = entity_cmds.id();
   commands.entity(container).add_child(entity);
}

// Заглушка для боевой статистики
pub struct CombatStats { pub attack: f32, pub defense: f32, pub speed: f32, pub health: f32 }
pub fn calculate_combat_stats(inv: &crate::plugins::metagame::PersistentInventory, db: &crate::plugins::items::ItemDatabase) -> CombatStats {
   let mut stats = CombatStats { attack: 0.0, defense: 0.0, speed: 0.0, health: 100.0 };

   for saved_item in &inv.items {
       if let Some(def) = db.items.get(&saved_item.item_id) {
           // Basic summing of stats
           stats.attack += def.attack;
           stats.defense += def.defense;
           stats.speed += def.speed;
           // Health usually isn't on items but if it was...
       }
   }

   stats
}
