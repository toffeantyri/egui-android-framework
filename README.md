# egui-android-framework

Легковесный MVI-фреймворк для запуска **egui**-приложений на Android с реактивной архитектурой в стиле Jetpack Compose.

**Однонаправленный поток данных.** Никакого polling. Состояние публикует себя само.

## Возможности

- **EGL + OpenGL ES 2.0** — прямой доступ к libEGL.so, без glutin
- **Реактивный StateStore\<T\>** — observable state на tokio::sync::watch
- **UiNotifier** — автоматическое пробуждение event loop при изменении состояния
- **Event-driven главный цикл** — poll_events + Wake + request_repaint, без фоновых потоков для UI
- **Decompose-style архитектура** — Application, Component, ChildStack + чистая View-функция
- **Android Lifecycle** — Create → Start → Resume → Pause → Stop → Destroy
- **Touch-ввод** — MotionEvent → egui::Event
- **Кнопка Back** — встроенная поддержка
- **Минимальный APK** — opt-level = "z", lto = true, strip = true

## Архитектурные принципы

Фреймворк реализует **MVI** (Model-View-Intent) с однонаправленным потоком данных:

```
Touch (кнопка)
  → View(state, ui) возвращает Vec<Message>
    → Component::handle(msg) → cmd_tx.send(msg)
      → Data Layer (фоновый поток)
        → store.update(|s| s.count += 1)
          → notify_tx.send(())          ← сигнал: «состояние изменилось»
            → UiNotifier::check()
              → ctx.request_repaint()
              → waker.wake()
                → poll_events получает Wake
                  → frame() → render(state) ← новый счётчик на экране
```

Каждый слой знает только о соседнем:

| Слой | Знает | Не знает |
|------|-------|----------|
| **View** | State → UI | Android, каналы, data layer |
| **Component** | View → Message | Data layer, бизнес-логика |
| **Application** | Component, Store, каналы | Android lifecycle, EGL |
| **Data Layer** | Store, Message | UI, Components, egui |
| **UiNotifier** | Context, Wake | Domain, Components, Reducer |

Никаких обратных связей. Никаких фоновых потоков для UI.
Polling запрещён — состояние само уведомляет Runtime.

## Быстрый старт

### 1. Cargo.toml

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

### 2. Определите сообщения и состояние

```rust
// msg.rs
/// Состояние (единственный источник истины).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct CounterState {
    pub count: u32,
}

/// Сообщение от View к data layer.
#[derive(Debug, Clone)]
pub enum Msg {
    Increment,
}
```

### 3. View — чистая функция

```rust
// view.rs
use crate::msg::Msg;

/// Чистая функция: читает State, рисует UI, возвращает Intent.
/// Не хранит состояние, не знает о каналах, не имеет побочных эффектов.
pub fn counter_view(state: &u32, ui: &mut egui::Ui) -> Vec<Msg> {
    let mut messages = vec![];

    ui.vertical_centered(|ui| {
        ui.heading(format!("Счётчик: {}", state));
        if ui.button("+1").clicked() {
            messages.push(Msg::Increment);
        }
    });

    messages
}
```

### 4. Component — преобразует Intent в Message

```rust
// component.rs
use egui_android_framework::{Component, LifecycleObserver, store::StateStore};
use crate::msg::{CounterState, Msg};
use crate::view::counter_view;

pub struct CounterComponent {
    count: u32,                          // snapshot
    store: StateStore<CounterState>,     // источник истины
}

impl CounterComponent {
    pub fn new(store: StateStore<CounterState>) -> Self {
        let count = store.state().count;
        Self { count, store }
    }

    /// Вызывается Application каждый кадр.
    pub fn sync_from_store(&mut self) {
        self.count = self.store.state().count;
    }
}

impl LifecycleObserver for CounterComponent {}

impl Component for CounterComponent {
    type State = u32;
    type Message = Msg;

    fn render(&self, ui: &mut egui::Ui) -> Vec<Self::Message> {
        counter_view(&self.count, ui)
    }

    fn handle(&mut self, msg: Self::Message) {
        // Component не меняет состояние — только логирует.
        // Сообщение уходит в data layer.
    }

    fn state(&self) -> &Self::State { &self.count }
}
```

### 5. Data Layer — бизнес-логика

```rust
// data_layer.rs
use egui_android_framework::store::StateStore;
use std::sync::mpsc;

pub fn data_layer_worker(
    cmd_rx: mpsc::Receiver<Msg>,
    store: StateStore<CounterState>,
    notify_tx: mpsc::Sender<()>,      // ← канал уведомлений
) {
    loop {
        match cmd_rx.recv() {
            Ok(Msg::Increment) => {
                // Единственная точка изменения состояния.
                store.update(|state| {
                    state.count = state.count.wrapping_add(1);
                });
                // Уведомить Runtime — он вызовет UiNotifier::check()
                let _ = notify_tx.send(());
            }
            Err(_) => break,
        }
    }
}
```

### 6. Application — корень DI

```rust
// app.rs
use std::sync::mpsc;
use egui_android_framework::{
    store::StateStore, AndroidWakeHandle, AppConfig, Application,
    Component, LifecycleObserver, UiNotifier,
};

pub struct CounterApp {
    root: CounterComponent,
    config: AppConfig,
    cmd_tx: mpsc::Sender<Msg>,
    notify_rx: mpsc::Receiver<()>,   // ← получает сигналы от data layer
}

impl LifecycleObserver for CounterApp {}

impl Application for CounterApp {
    type RootComponent = CounterComponent;

    fn create() -> Self {
        let config = AppConfig::default();
        let store = StateStore::new(CounterState { count: 0 });

        let (cmd_tx, cmd_rx) = mpsc::channel::<Msg>();
        let (notify_tx, notify_rx) = mpsc::channel::<()>();

        let store_for_worker = store.clone_state();

        // Запускаем data layer в фоне
        std::thread::spawn(move || {
            data_layer_worker(cmd_rx, store_for_worker, notify_tx);
        });

        // Компонент получает копию store (разделяемый watch-канал)
        let root = CounterComponent::new(store.clone_state());

        Self { root, config, cmd_tx, notify_rx }
    }

    fn root(&mut self) -> &mut CounterComponent { &mut self.root }
    fn root_ref(&self) -> &CounterComponent { &self.root }
    fn config(&self) -> &AppConfig { &self.config }
    fn config_mut(&mut self) -> &mut AppConfig { &mut self.config }

    /// Runtime вызывает после инициализации EGL.
    /// Возвращает UiNotifier, который проверяется в главном цикле.
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

        let mut messages = vec![];
        let output = egui_ctx.run(raw_input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                messages = self.root.render(ui);
            });
        });

        for msg in messages {
            self.root.handle(msg.clone());
            let _ = self.cmd_tx.send(msg);
        }
        output
    }
}
```

### 7. Точка входа

```rust
// lib.rs
pub mod app;
pub mod component;
pub mod data_layer;
pub mod msg;
pub mod view;

#[cfg(target_os = "android")]
use egui_android_framework::android::run;

#[cfg(target_os = "android")]
#[no_mangle]
pub fn android_main(app: android_activity::AndroidApp) {
    run::<app::CounterApp>(app);
}
```

### 8. Сборка и запуск

```bash
cargo install xbuild
cd my-egui-app
x run --device adb:XXXXXXXX
```

Полный пример — [`examples/counter2`](examples/counter2).

## API: ключевые типы

### Application

```rust
pub trait Application: LifecycleObserver + Sized + 'static {
    type RootComponent: Component;
    fn create() -> Self;
    fn root(&mut self) -> &mut Self::RootComponent;
    fn root_ref(&self) -> &Self::RootComponent;
    fn config(&self) -> &AppConfig;
    fn config_mut(&mut self) -> &mut AppConfig;
    fn create_notifier(&mut self, ctx: &egui::Context, wake: AndroidWakeHandle) -> Option<UiNotifier>;
    fn frame(&mut self, egui_ctx: &egui::Context, raw_input: egui::RawInput) -> egui::FullOutput;
}
```

### Component

```rust
pub trait Component: LifecycleObserver + Send + 'static {
    type State: 'static;
    type Message: 'static;
    fn render(&self, ui: &mut egui::Ui) -> Vec<Self::Message>;
    fn handle(&mut self, msg: Self::Message);
    fn state(&self) -> &Self::State;
}
```

### StateStore\<T\>

```rust
pub struct StateStore<T> { /* на tokio::sync::watch */ }

impl<T: Clone + Send + Sync + 'static> StateStore<T> {
    pub fn new(initial: T) -> Self;
    pub fn update(&self, f: impl FnOnce(&mut T));
    pub fn dispatch<M>(&self, msg: M, reducer: fn(&mut T, M));
    pub fn state(&self) -> T;
    pub fn subscribe(&self) -> watch::Receiver<T>;
    pub fn clone_state(&self) -> Self;
}
```

### UiNotifier

```rust
pub struct UiNotifier { /* mpsc::Receiver<()> */ }

impl UiNotifier {
    pub fn new(ctx: egui::Context, wake: Option<AndroidWakeHandle>, rx: mpsc::Receiver<()>) -> Self;
    pub fn check(&mut self);
}
```

## Интеграция

### Каналы

| Канал | Тип | Откуда → Куда |
|-------|-----|---------------|
| `cmd_tx / cmd_rx` | `mpsc::channel::<Msg>()` | Component → Data Layer |
| `notify_tx / notify_rx` | `mpsc::channel::<()>()` | Data Layer → UiNotifier |
| `store` | `StateStore<T>` (watch) | Data Layer ↔ Component |

### Создание каналов (в Application::create())

```rust
let store = StateStore::new(MyState::default());
let (cmd_tx, cmd_rx) = mpsc::channel::<Msg>();
let (notify_tx, notify_rx) = mpsc::channel::<()>();

let store_for_worker = store.clone_state();   // data layer
let component = MyComponent::new(store.clone_state());  // UI

std::thread::spawn(move || {
    my_data_layer_worker(cmd_rx, store_for_worker, notify_tx);
});
```

## Планы

| Возможность | Статус |
|---|---|
| StateStore + UiNotifier | ✅ Реализовано |
| Component + ViewFn | ✅ Реализовано |
| ChildStack (push/pop/replace) | ✅ Реализовано |
| Lifecycle (LifecycleObserver) | ✅ Реализовано |
| Application как корень DI | ✅ Реализовано |
| run() — событийный главный цикл | ✅ Реализовано |
| Навигация: ChildStack + ComponentFactory | 🚧 В разработке |
| ComponentContext (навигация + DI) | 🚧 В разработке |
| Сохранение состояния при повороте | 🔜 |
| BottomSheet / Modal / Dialog компоненты | 🔜 |
| Анимации переходов | 🔜 |

## Тесты

55 тестов: StateStore (10), Integration (22), UiNotifier (5), ChildStack (6),
Lifecycle (1), Application (4), EguiSubscriber (6), Error (1).

## Зависимости

| Крейт | Назначение |
|---|---|
| `egui` 0.31 | GUI |
| `egui_glow` 0.31 | Рендеринг через OpenGL |
| `glow` 0.16 | OpenGL context |
| `android-activity` 0.6 | NativeActivity |
| `ndk` 0.9 | NDK bindings (NativeWindow) |
| `libc` 0.2 | dlopen/dlsym для GL |
| `tokio` 1 (sync) | watch::channel для StateStore |
| `log` 0.4 + `android_logger` 0.14 | Логирование |

## Лицензия

MIT или Apache 2.0
