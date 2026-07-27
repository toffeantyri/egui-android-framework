# Проблемы

## 2. Type erasure — runtime ошибка вместо compile-time

```
Проблема: Type erasure через Box<dyn Any> теряет типобезопасность

Файл: crates/core/src/component_node.rs

Суть:
  fn handle_dyn(&mut self, msg: Box<dyn std::any::Any + Send>) {
      if let Ok(typed) = msg.downcast::<T::Message>() {
          crate::Component::handle(self, *typed);
      } else {
          log::error!("ожидался {}, получен неизвестный", ...);
      }
  }

  Неправильный тип сообщения = тихий log::error в рантайме,
  а не ошибка компиляции.

  DynDispatcher (crates/runtime/src/dyn_dispatcher.rs) упаковывает
  M в Box<dyn Any + Send> — вся типобезопасность теряется на
  границе View → Component.

Влияние:
  - Ошибки маршрутизации сообщений обнаруживаются только в рантайме
  - Нет compile-time гарантии, что View шлёт правильный тип

Затрагиваемые слои: core, runtime
Риск: средний — требует изменения публичного API
```

## 4. Гибридные Constraints — два источника правды

```
Проблема: Constraints хранятся в двух местах одновременно

Файл: crates/core/src/ui_wrapper.rs

Суть:
  pub enum UiWrapper<'a> {
      Borrowed(&'a mut egui::Ui, Constraints),  // ← поле
      Owned(Box<egui::Ui>, Constraints),        // ← поле
  }

  fn read_cx(ui: &egui::Ui) -> Constraints {
      ui.ctx().data(|d| d.get_temp::<Constraints>(cx_key()).unwrap_or_default())
  }
  fn write_cx(ui: &egui::Ui, constraints: Constraints) {
      ui.ctx().data_mut(|d| d.insert_temp(cx_key(), constraints));
  }

  Поле UiWrapper + Context::data() = два источника правды.
  Глобальный ключ cx_key() — один на весь Context, не привязан к Ui.
  При вложенных UiWrapper последний write_cx() перезаписывает предыдущий.
  Зависимость от internals egui (IdTypeMap).

Влияние:
  - Рассинхронизация поля и Context::data() при прямом доступе через Deref
  - Хрупкость при обновлении egui
  - Невозможность параллельных веток с разными Constraints

Затрагиваемые слои: core
Риск: средний — влияет на все контейнеры и модификаторы
```

## 5. remember() мутирует состояние внутри рендера

```
Проблема: remember() нарушает декларацию «store.update() — единственная точка мутации»

Файл: crates/ui/src/remember.rs

Суть:
  pub fn set(&self, new_value: T) {
      *self.value.write().expect("...") = new_value.clone();
      self.persist(&new_value);  // ← мутация ctx.data_mut внутри рендера
  }

  on_click_with() делает то же самое — вызывает closure в момент рендера.

  arch.md декларирует: «store.update() — единственная точка изменения состояния».
  Но remember() и on_click_with() мутируют состояние напрямую.

Влияние:
  - Архитектурное противоречие между декларацией и реализацией
  - Неочевидно для разработчика, какие данные где мутируются

Затрагиваемые слои: ui, документация (arch.md)
Риск: нулевой при документировании, высокий при изменении кода
```



## 7. Platform-абстракция минимальна

```
Проблема: Крейт platform — пустые трейты, кросс-платформенность не реализована

Файл: crates/platform/src/platform.rs

Суть:
  pub trait Platform {
      type Window: Clone + Send + 'static;
      type InputEvent: Send + 'static;
      type Error: std::fmt::Debug + Send + 'static;
      fn run<A>(app: A, config: PlatformConfig) -> Result<(), Self::Error>;
  }

  5 файлов в platform/ (platform.rs, event.rs, frame.rs, config.rs, waker.rs),
  но реальный код — только в platform-android/.
  Нет platform-desktop, platform-web.
  run() захардкожен на AndroidApp.

Влияние:
  - Кросс-платформенность заявлена, но не достигнута
  - Абстракция не проверяется вторым потребителем

Затрагиваемые слои: platform, platform-android, новый platform-desktop
Риск: высокий — требует нового backend
```


## 9. Blanket-impl конфликт решён обёрткой

```
Проблема: PersistentComponent<T> — вынужденная обёртка из-за ограничений Rust

Файл: crates/core/src/persistent_state.rs

Суть:
  // Rust запрещает два blanket-impl:
  // impl<T: Component> ComponentNode for T { save_state = None }
  // impl<T: Component + PersistentState> ComponentNode for T { save_state = Some }
  //
  // Решение: compositional wrapper
  pub struct PersistentComponent<T> { pub inner: T }

  Каждый persistent-компонент нужно вручную оборачивать в фабрике:
  Route::State => Box::new(PersistentComponent::new(StateScreen::new())),

  Без обёртки save_state() вернёт None — состояние не сохранится.
  Ошибка невидима до рантайма.

Влияние:
  - Лишний boilerplate в каждой фабрике
  - Легко забыть обёртку — тихая потеря состояния

Затрагиваемые слои: core, macros
Риск: средний — изменение публичного API
```

---

# Задачи (решения)


## Задача 3: Документировать двухуровневую модель мутации

```
Задача: Задокументировать два уровня мутации состояния

Что сделать:
  1. В arch.md добавить раздел «Двухуровневая модель мутации»:

     Уровень 1 (MVI / бизнес-данные):
       Intent → Message → Reducer → State → UI
       store.update() — единственная точка мутации
       Примеры: счётчик, список товаров, auth-токен
       Сохраняется при kill/restore через PersistentState

     Уровень 2 (Local / UI-состояние):
       remember() / on_click_with()
       Arc<RwLock<T>> в IdTypeMap
       НЕ сохраняется при kill/restore
       Примеры: expanded/collapsed, позиция скролла, текст ввода

     Правило: если данные нужны после пересоздания Activity —
     это бизнес-данные → MVI. Если нет — UI-состояние → remember().

  2. В guide.md обновить раздел «Правила»:
     Заменить «store.update() — единственная точка изменения состояния»
     на «store.update() — единственная точка изменения БИЗНЕС-состояния»

  3. В arch.md обновить контракт:
     «UI никогда самостоятельно не изменяет State» →
     «UI никогда самостоятельно не изменяет бизнес-State.
      Локальное UI-состояние (remember) — исключение.»

Проверка:
  Ревью документации, cargo check (код не меняется)

Затрагиваемые слои: документация
Оценка: 2 часа
```

## Задача 4: Убрать гибридное хранение Constraints

```
Задача: Оставить один источник правды для Constraints

Вариант A (рекомендуемый): только Context::data(), убрать поле
  1. В UiWrapper убрать поле Constraints из обоих вариантов enum
  2. constraints() → всегда read_cx(self.ui)
  3. set_constraints() → всегда write_cx(self.ui, c)
  4. new() → write_cx + UiWrapper без поля
  5. new_unconstrained() → read_cx + UiWrapper без поля

  Плюс: один источник, нет рассинхронизации
  Минус: хэш-таблица на каждый доступ (некритично для 60 FPS)

Вариант B: только поле, убрать Context::data()
  1. Убрать read_cx/write_cx
  2. Frame::show() → передавать Constraints через UiBuilder или параметр
  3. Контейнеры явно передают Constraints детям

  Плюс: быстрее
  Минус: ломает совместимость с Frame::show(), ScrollArea::show()

Что сделать (вариант A):
  1. Изменить UiWrapper в crates/core/src/ui_wrapper.rs
  2. Обновить все контейнеры (Column, Row, Stack, LazyColumn)
  3. Обновить Modifier::apply_recursive
  4. Обновить тесты в crates/ui/tests/

Проверка:
  cargo test --workspace
  cargo test -p egui-android-ui  # layout_tests, widget_tests

Затрагиваемые слои: core, ui
Оценка: 3 дня
```

## Задача 5: Макрос #[derive(ComponentNode)] для устранения обёртки

```
Задача: Убрать необходимость вручную оборачивать в PersistentComponent

Что сделать:
  1. В crates/macros/src/lib.rs добавить proc-macro:
     #[derive(Component, ComponentNode)]
     #[persistent_fields(counter, label)]
     struct MyScreen { ... }

     Макрос ComponentNode генерирует:
     impl ComponentNode for MyScreen {
         fn save_state(&self) -> Option<Box<dyn Any + Send>> {
             PersistentState::save_to_boxed(self)
         }
         fn restore_state(&mut self, state: Box<dyn Any + Send>) {
             PersistentState::restore_from_boxed(self, state);
         }
         // render, handle_dyn, as_any, as_any_mut — делегирование
     }

     Это конкретный impl (не blanket) — не конфликтует с blanket-impl.

  2. В фабриках убрать обёртку:
     Было: Box::new(PersistentComponent::new(StateScreen::new()))
     Стало: Box::new(StateScreen::new())

  3. PersistentComponent<T> оставить для обратной совместимости,
     пометить #[deprecated]

Проверка:
  cargo test --workspace
  cargo test -p egui-android-macros  # integration tests
  cargo test -p egui-android-navigation  # child_stack_save_tests

Затрагиваемые слои: macros, core, navigation, examples
Оценка: 3 дня
```

## Задача 8: Enum-based dispatch для compile-time safety

```
Задача: Заменить Box<dyn Any> downcast на типизированный enum

Что сделать:
  1. Генерировать enum сообщений макросом из Route:

     // Генерируется из enum Route { State, Widgets, ... }
     enum ScreenMsg {
         StateScreen(state_screen::Msg),
         WidgetsScreen(widgets::Msg),
         // ...
     }

  2. ComponentNode получает типизированный handle:
     fn handle_enum(&mut self, msg: ScreenMsg) {
         match msg {
             ScreenMsg::StateScreen(m) => self.handle(m),
             // ...
         }
     }

  3. DynDispatcher остаётся для динамических случаев (плагины),
     но основной путь — типизированный enum

  4. Ошибка в типе сообщения = compile error

Проверка:
  cargo check --workspace
  cargo test --workspace

Затрагиваемые слои: core, runtime, macros, navigation
Оценка: 2 недели
```

## Задача 9: Desktop/Web платформы

```
Задача: Реализовать кросс-платформенность через platform-desktop

Что сделать:
  1. Создать crates/platform-desktop/:
     - winit event loop
     - glow renderer (уже используется в platform-android)
     - run<A: Application>() с тем же контрактом

  2. Выделить общий код из platform-android в platform:
     - RunState::tick() → общий для всех платформ
     - GraphicsPipeline → общий (glow)
     - Input processing → адаптер под winit events

  3. platform-android остаётся Android-специфичным:
     - EGL, GameActivity, JNI, IME

  4. Проверить, что Application trait не содержит Android-специфики

Проверка:
  cargo check --workspace
  cargo run -p example-counter  # desktop
  cargo ndk build -p example-counter  # android

Затрагиваемые слои: platform, platform-android, новый platform-desktop
Оценка: 2 месяца
```

## Задача 10: Гибридная MVI модель — документация

```
Задача: Явно задокументировать гибридную MVI + Local State модель

Что сделать:
  1. В arch.md добавить диаграмму:

     ┌─────────────────────────────────────────────────┐
     │                  Framework                       │
     │                                                  │
     │  ┌──────────────┐    ┌──────────────────────┐   │
     │  │  MVI Layer   │    │   Local State Layer  │   │
     │  │              │    │                      │   │
     │  │  Intent      │    │  remember()          │   │
     │  │  Message     │    │  on_click_with()     │   │
     │  │  Reducer     │    │  Arc<RwLock<T>>      │   │
     │  │  State       │    │                      │   │
     │  │  store.      │    │  Не сохраняется      │   │
     │  │  update()    │    │  при kill/restore    │   │
     │  └──────┬───────┘    └──────────┬───────────┘   │
     │         │                       │               │
     │         ▼                       ▼               │
     │  ┌─────────────────────────────────────────┐    │
     │  │              UI (egui)                   │    │
     │  │  Component::render(ui, dispatch)         │    │
     │  └─────────────────────────────────────────┘    │
     └─────────────────────────────────────────────────┘

  2. Аналогия с Jetpack Compose:
     MVI Layer = ViewModel + StateFlow
     Local State = remember() / rememberSaveable()

  3. Обновить guide.md:
     - Раздел «Упрощённая MVI-модель» → «Гибридная модель»
     - Добавить правило: «remember() — для UI-состояния,
       store.update() — для бизнес-данных»

Проверка:
  Ревью документации, код не меняется

Затрагиваемые слои: документация
Оценка: 2 часа
```

---
