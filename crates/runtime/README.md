# egui-android-runtime

Runtime слой: Application, Dispatcher, StateStore, UiNotifier, AppConfig.

## Типы

- **`Application`** — корень DI. Владеет RootComponent, каналами, конфигом
- **`Dispatcher<M>`** — абстракция над `mpsc::Sender`. Clone, отправляет сообщения
- **`StateStore<T>`** — реактивное состояние на `tokio::sync::watch`
- **`UiNotifier`** — проверяет канал уведомлений, вызывает `request_repaint()`
- **`AppConfig`** — настройки: log_tag, target_fps

## Пример

```rust
use egui_android_runtime::{Application, Dispatcher, StateStore, AppConfig};

struct MyApp { store: StateStore<u32> }

impl Application for MyApp {
    type RootComponent = MyComponent;

    fn create() -> Self {
        Self { store: StateStore::new(0) }
    }

    fn frame(&mut self, egui_ctx: &egui::Context, raw_input: egui::RawInput) -> egui::FullOutput {
        self.root.sync_from_store();
        let (dispatcher, receiver) = Dispatcher::new();
        let output = egui_ctx.run_ui(raw_input, |ctx| {
            CentralPanel::default().show(ctx, |ui| {
                self.root.render(ui, &dispatcher);
            });
        });
        for msg in receiver.try_iter() { self.root.handle(msg); }
        output
    }
}
```
