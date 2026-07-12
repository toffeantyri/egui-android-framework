---
name: egui-android-integration
description: Инструкция по быстрой интеграции egui-android-framework для создания Android-приложений на Rust + egui. Используй при создании нового проекта, добавлении экранов, настройке data layer или миграции с другой архитектуры.
---

# Интеграция egui-android-framework

Этот skill содержит пошаговые инструкции и шаблоны для создания Android-приложений
на Rust с использованием egui и egui-android-framework.

## Справочник API

### Stack (контейнер-наложение)

**Расположение:** `egui_android_framework::ui::containers::Stack`  
**Align:** `egui_android_framework::ui::containers::Align`

```rust
use egui_android_framework::ui::containers::{Stack, Align};

Stack::new()
    .add(child1)
    .add_with_align(child2, Align::Center)  // align отложен до pub set_cursor
    .show(ui, dispatch);
```

**Правила:**
- Принимает список детей (builder pattern), не один closure
- Wrap-content: consum = max(children), не sum
- Overlay: дети накладываются, не идут последовательно
- Двухфазный measure→layout (как Compose Box)
- fill_max_width работает через модификатор ребёнка

### ButtonColors и визуальный отклик

**Расположение:** `egui_android_framework::ui::widgets::ButtonColors`

```rust
use egui_android_framework::ui::{widgets::Button, theme::Colors};

// 1. Дефолтные цвета (синий/оранжевый) — автоматически
Button::new("Нажми").render(ui, dispatch);

// 2. Цвета по теме — pressed вычисляется автоматически
Button::new("Тема")
    .theme_colors(c.primary)  // c — из Theme::current()
    .text_color(c.on_primary)
    .render(ui, dispatch);

// 3. Кастомные цвета (полный контроль)
Button::new("Кастом")
    .colors(Colors::LIGHT_GREEN, Colors::EMERALD)
    .text_color(Colors::WHITE)
    .render(ui, dispatch);
```

**Правила `theme_colors`:**
- Для светлых цветов (Value > 0.5) — затемняет на 30% для pressed
- Для тёмных цветов (Value <= 0.5) — осветляет на 40% для pressed
- Гарантирует normal ≠ pressed для любой темы

## Шаблон проекта

Новое приложение состоит из следующих файлов:

```
my-app/
├── AndroidManifest.xml
├── Cargo.toml
├── src/
│   ├── lib.rs              # Точка входа android_main
│   ├── app.rs              # Application: DI, каналы
│   ├── component.rs        # Component: render + handle
│   ├── data_layer.rs       # Data Layer: бизнес-логика
│   ├── msg.rs              # Типы: State, Message
│   └── view.rs             # View: чистая функция
```

## Cargo.toml (шаблон)

```toml
[package]
name = "my-egui-app"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
egui-android-framework = { git = "https://gitverse.ru/Tofy3434/egui-android-framework" }
egui = "0.31"
log = "0.4"

[target.'cfg(target_os = "android")'.dependencies]
android_logger = "0.14"
android-activity = "0.6"
```

## Пошаговая инструкция

### Шаг 1: Определить типы (msg.rs)

```rust
/// Состояние — хранится в StateStore.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct AppState {
    // поля состояния
}

/// Сообщение от View к data layer.
#[derive(Debug, Clone)]
pub enum Msg {
    // варианты сообщений
}
```

**Правила:**
- `AppState` должен быть `Clone + Send + Sync + 'static`
- `Msg` должен быть `Send + 'static`
- Никакой логики в `msg.rs` — только типы

---

### Шаг 2: View — чистая функция с Dispatcher (view.rs)

```rust
use egui_android_framework::runtime::Dispatcher;
use egui_android_framework::ui::{
    containers::Column,
    modifier::{Modifier, ModifierDsl},
    widgets::{Button, Spacer, Text, Widget},
    UiWrapper,
};

use crate::msg::{AppState, Msg};

/// View-функция. Сообщения отправляются через dispatch в момент события.
pub fn my_view(state: &AppState, ui: &mut UiWrapper, dispatch: &Dispatcher<Msg>) {
    Column::new().show(ui, dispatch, |ui, dispatch| {
        Text::new("Привет, мир!").render(ui, dispatch);
        Spacer::new(8.0).render(ui, dispatch);

        Button::new("Действие")
            .on_click(Msg::SomeAction)
            .modifier(Modifier::new().padding(8.0))
            .render(ui, dispatch);
    });
}
```

**Правила:**
- View ничего не знает о каналах, Android, data layer
- View НЕ хранит состояние (кроме локального `remember`)
- Сообщения отправляются через `dispatch.dispatch(msg)` в момент события
- Никаких `state.value = ...` внутри View
- View принимает `&mut UiWrapper` (не `&mut egui::Ui`) — даёт доступ к Constraints
- Контейнеры используют `.show(ui, dispatch, |ui, dispatch| { ... })` (Compose-like замыкания)
- Модификаторы применяются через `.modifier(Modifier::new().padding(8.0).background(red))`

**Для локального UI-состояния используется `remember`:**

```rust
use egui_android_framework::ui::remember;

pub fn my_view(state: &AppState, ui: &mut UiWrapper, dispatch: &Dispatcher<Msg>) {
    Column::new().show(ui, dispatch, |ui, dispatch| {
        let count = remember(ui, "counter", || 0i32);

        Text::new(format!("Счётчик: {}", count.get())).render(ui, dispatch);

        // on_click — MVI-поток (через Component → Data Layer → StateStore)
        Button::new("Сохранить").on_click(Msg::Save).render(ui, dispatch);

        // on_click_with — локальное UI-действие (только remember)
        Button::new("+1").on_click_with({
            let count = count.clone();
            move |_ui, _dispatch| count.modify(|c| *c += 1)
        }).render(ui, dispatch);
    });
}
```

**Правила `remember`:**
- Методы `set()`, `modify()` принимают `&self` — не нужно `mut`
- `Clone` через `Arc::clone()` — все клоны разделяют одно состояние
- Между кадрами состояние сохраняется (хранится в `egui::Context::data()`)

---

### Шаг 3: Component (component.rs)

```rust
use egui_android_framework::{
    core::{Component, LifecycleObserver, UiWrapper},
    runtime::{Dispatcher, StateStore},
};
use crate::msg::{AppState, Msg};
use crate::view::my_view;

pub struct MyComponent {
    state_snapshot: AppState,
    store: StateStore<AppState>,
}

impl MyComponent {
    pub fn new(store: StateStore<AppState>) -> Self {
        let snapshot = store.state();
        Self { state_snapshot: snapshot, store }
    }

    /// Вызывается Application::frame() каждый кадр.
    pub fn sync_from_store(&mut self) {
        self.state_snapshot = self.store.state();
    }
}

impl LifecycleObserver for MyComponent {}

impl Component for MyComponent {
    type State = AppState;
    type Message = Msg;

    fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<Self::Message>) {
        let mut wrapper = UiWrapper::new_unconstrained(ui);
        my_view(&self.state_snapshot, &mut wrapper, dispatch);
    }

    fn handle(&mut self, _msg: Self::Message) {
        // Только логирование. Сообщение уходит в data layer.
    }

    fn state(&self) -> &Self::State { &self.state_snapshot }
}
```

**Правила:**
- `render()` принимает `&Dispatcher<M>` — сообщения диспатчатся в момент события
- Component не меняет состояние
- Component не обращается к data layer напрямую
- `sync_from_store()` вызывается перед каждым render

---

### Шаг 4: Data Layer (data_layer.rs)

```rust
use egui_android_framework::runtime::StateStore;
use std::sync::mpsc;

use crate::msg::{AppState, Msg};

pub fn my_data_layer(
    cmd_rx: mpsc::Receiver<Msg>,
    store: StateStore<AppState>,
    notify_tx: mpsc::Sender<()>,
) {
    loop {
        match cmd_rx.recv() {
            Ok(msg) => {
                // Обработка сообщения
                store.update(|state| {
                    // Изменяем состояние
                });
                // Уведомляем Runtime об изменении
                let _ = notify_tx.send(());
            }
            Err(_) => break,
        }
    }
}
```

**Правила:**
- `store.update()` — ЕДИНСТВЕННОЕ место изменения состояния
- После каждого `update()` — `notify_tx.send(())`
- Data layer не знает про UI, Components, egui

---

### Шаг 5: Application (app.rs)

```rust
use std::sync::mpsc;
use egui_android_framework::{
    runtime::{AndroidWakeHandle, AppConfig, Application, Dispatcher, StateStore, UiNotifier},
    core::{Component, LifecycleObserver, UiWrapper},
};

use crate::component::MyComponent;
use crate::data_layer::my_data_layer;
use crate::msg::{AppState, Msg};

pub struct MyApp {
    root: MyComponent,
    config: AppConfig,
    cmd_tx: mpsc::Sender<Msg>,
    notify_rx: mpsc::Receiver<()>,
}

impl LifecycleObserver for MyApp {}

impl Application for MyApp {
    type RootComponent = MyComponent;

    fn create() -> Self {
        let config = AppConfig::default();
        let store = StateStore::new(AppState::default());
        let (cmd_tx, cmd_rx) = mpsc::channel::<Msg>();
        let (notify_tx, notify_rx) = mpsc::channel::<()>();

        let store_for_worker = store.clone_state();
        std::thread::spawn(move || {
            my_data_layer(cmd_rx, store_for_worker, notify_tx);
        });

        let root = MyComponent::new(store.clone_state());

        Self { root, config, cmd_tx, notify_rx }
    }

    fn root(&mut self) -> &mut MyComponent { &mut self.root }
    fn root_ref(&self) -> &MyComponent { &self.root }
    fn config(&self) -> &AppConfig { &self.config }
    fn config_mut(&mut self) -> &mut AppConfig { &mut self.config }

    fn create_notifier(
        &mut self,
        ctx: &egui::Context,
        wake: AndroidWakeHandle,
    ) -> Option<UiNotifier> {
        let rx = std::mem::replace(&mut self.notify_rx, mpsc::channel().1);
        Some(UiNotifier::new(ctx.clone(), Some(wake), rx))
    }

    fn frame(&mut self, egui_ctx: &egui::Context, raw_input: egui::RawInput) -> egui::FullOutput {
        self.root.sync_from_store();

        let (dispatcher, receiver) = Dispatcher::new();

        let full_output = egui_ctx.run_ui(raw_input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let mut wrapper = UiWrapper::new_unconstrained(ui);
                self.root.render(&mut wrapper, &dispatcher);
            });
        });

        for msg in receiver.try_iter() {
            self.root.handle(msg.clone());
            let _ = self.cmd_tx.send(msg);
        }

        full_output
    }
}
```

**Правила:**
- `Dispatcher` создаётся каждый кадр и живёт один кадр
- `receiver.try_iter()` drain'ит все сообщения после render
- Три канала: `cmd_tx/cmd_rx`, `notify_tx/notify_rx`, `store`
- `store.clone_state()` — для data layer и component (разделяют watch-канал)
- `create_notifier()` вызывается Runtime после EGL init
- В `frame()`: `sync_from_store()` → `Dispatcher::new()` → `render()` → `handle()` + `cmd_tx.send()`

---

### Шаг 6: Точка входа (lib.rs)

```rust
pub mod app;
pub mod component;
pub mod data_layer;
pub mod msg;
pub mod view;

#[cfg(target_os = "android")]
use egui_android_framework::platform_android::run;

#[cfg(target_os = "android")]
#[no_mangle]
pub fn android_main(app: android_activity::AndroidApp) {
    run::<app::MyApp>(app);
}
```

---

## Поток данных (диаграмма)

```
Пользователь нажимает кнопку
          │
          ▼
View: dispatch.dispatch(Msg::Increment)    ← в момент клика
          │
          ▼
Receiver (mpsc) накапливает сообщения      ← после render: drain
          │
          ▼
Component::handle(msg)                     ← отправляет команду
          │
          ▼
cmd_tx.send(msg) ─────────────────► Data Layer (фоновый поток)
                                        │
                                        ▼
                               store.update(|s| { ... })  ← ЕДИНСТВЕННАЯ ТОЧКА
                                        │
                                        ▼
                               notify_tx.send(())
                                        │
                                        ▼
                               UiNotifier::check()
                                 ├─ ctx.request_repaint()
                                 └─ waker.wake()
                                        │
                                        ▼
                               poll_events() → Wake
                                        │
                                        ▼
                               frame() → sync_from_store()
                                        │
                                        ▼
                               render(state) — новый UI
```

## Локальное UI-состояние (remember)

```
Клик по кнопке с on_click_with
          │
          ▼
count.modify(|c| *c += 1)                ← напрямую меняет Arc<RwLock<T>>
          │
          ▼
Те же кадр: UI отображает новое значение   ← без MVI-потока
```

**Когда использовать `on_click` (MVI):**
- Состояние влияет на бизнес-логику
- Состояние должно сохраняться между сессиями
- Состояние используется в нескольких компонентах

**Когда использовать `on_click_with` / `clickable_with` (remember):**
- Состояние локально для одного UI-элемента
- Состояние временное (сбрасывается при пересоздании контекста)
- Состояние не влияет на бизнес-логику

## Проверка архитектуры (чеклист)

Перед завершением интеграции убедись:

- [ ] View — чистая функция, без побочных эффектов
- [ ] Component не меняет состояние, не знает о data layer
- [ ] `store.update()` — единственная точка изменения состояния
- [ ] После каждого `update()` вызывается `notify_tx.send(())`
- [ ] `Dispatcher` создаётся каждый кадр в `frame()`
- [ ] Сообщения отправляются через `dispatch.dispatch()` в момент события
- [ ] Три канала: cmd, notify, store — каждый со своей ответственностью
- [ ] `create_notifier()` реализован в Application
- [ ] `sync_from_store()` вызывается в начале `frame()`
- [ ] `android_main` вызывает `run::<MyApp>(app)`

## Распространённые ошибки

1. **Не вызывать `notify_tx.send(())` после `store.update()`**
   → UiNotifier не узнает об изменении, UI не обновится

2. **Вызывать `store.update()` не из data layer**
   → нарушение MVI — только data layer меняет состояние

3. **Забыть `sync_from_store()` в `frame()`**
   → компонент показывает устаревшее состояние

4. **Использовать глобальные переменные/OnceLock для каналов**
   → не сбрасываются между запусками, приводит к SIGABRT

5. **Вызывать `poll()` в `frame()`**
   → не нужен, всё реактивно

6. **Создавать фоновые потоки для UI**
   → всё в главном потоке, data layer — единственный фоновый

7. **Пытаться изменить remember через `on_click(msg)`**
   → remember — локальное UI-состояние, для него есть `on_click_with`

## Сборка и запуск

```bash
cargo install xbuild
cd my-app
x run --device adb:XXXXXXXX
```
