# egui-android-runtime

Runtime слой: главный цикл приложения, диспатчер сообщений, реактивное состояние.

## Проблема

egui не предоставляет:
- единого корня приложения (DI, каналы, lifecycle)
- системы отправки сообщений от View к Component
- реактивного состояния с автоматическим уведомлением UI об изменениях
- инфраструктуры для data layer (фоновые потоки, каналы)

Этот крейт реализует всю рантайм-инфраструктуру.

## Состав

| Тип | Описание |
|---|---|
| **`Application`** | Корень DI. Владеет RootComponent, каналами, конфигом. Методы: `create()`, `frame()`, `root()`, `on_back_pressed()`, `request_destroy()`, lifecycle |
| **`Dispatcher<M>`** | Абстракция над `std::sync::mpsc::Sender`. `Clone`, `dispatch(msg)` — отправляет сообщение в момент UI-события |
| **`StateStore<T>`** | Реактивное состояние на `tokio::sync::watch`. `update(f)` — атомарно изменить состояние + уведомить. `dispatch(msg, reducer)` — MVI-диспатч |
| **`UiNotifier`** | Проверяет `mpsc::Receiver<()>`, при сигнале вызывает `ctx.request_repaint()` и `waker.wake()` |
| **`AppConfig`** | Настройки: `log_tag`, `target_fps` |
| **`DataLayerHandle`** | Handle для отправки команд в data layer (заглушка) |

### Поток данных

```
View (клик) → Dispatcher::dispatch(msg)
  → Receiver (mpsc) — накопление за кадр
    ← после render: drain receiver → Component::handle(msg)
      → команда в data layer (фоновый поток)
        → StateStore::update(|s| ...) — единственная точка изменения состояния
          → notify_tx.send(())
            → UiNotifier::check() → request_repaint() + waker.wake()
              → poll_events() → frame() → render(state, &dispatcher) → новый UI
```

### Типовой frame()

```rust
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
        self.root.handle(msg);
    }

    full_output
}
```

После `frame()` рантайм проверяет `request_destroy()` — если `true`, цикл завершается.

## Зависимости

- `egui` — GUI
- `tokio` (sync) — watch-каналы для StateStore
- `thiserror` — AppError derive
- `log`

## Пример

```rust
use egui_android_runtime::{Application, Dispatcher, StateStore, AppConfig};

struct MyApp { root: MyComponent, store: StateStore<AppState> }

impl Application for MyApp {
    type RootComponent = MyComponent;

    fn create() -> Self {
        let store = StateStore::new(AppState::default());
        Self { root: MyComponent::new(store.clone_state()), store }
    }

    fn frame(&mut self, egui_ctx: &egui::Context, raw_input: egui::RawInput) -> egui::FullOutput {
        self.root.sync_from_store();
        let (dispatcher, receiver) = Dispatcher::new();
        let output = egui_ctx.run_ui(raw_input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                self.root.render(ui, &dispatcher);
            });
        });
        for msg in receiver.try_iter() { self.root.handle(msg); }
        output
    }

    fn on_back_pressed(&mut self) {
        self.root.on_back();
    }
}
```
