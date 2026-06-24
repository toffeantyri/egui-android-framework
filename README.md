# egui-android-framework

Легковесный фреймворк для запуска GUI-приложений на **egui** под Android.

## Возможности

- **EGL + OpenGL ES 2.0** — прямая работа с libEGL.so, без `glutin`
- **MVVM-архитектура** — `Application`, `Activity`, `ViewModel` с каналами для data layer
- **Android Lifecycle** — полная поддержка `Create → Start → Resume → Pause → Stop → Destroy`
- **Touch-ввод** — трансляция `MotionEvent` в `egui::Event`
- **Кнопка Back** — обрабатывается через `Activity::on_back_pressed()`
- **Смена конфигурации** — пересоздание EGL-поверхности при повороте/изменении окна
- **Минимальный размер APK** — `opt-level = "z"`, `lto = true`, `strip = true`

## Быстрый старт

### 1. Зависимости

```toml
[dependencies]
egui-android-framework = { git = "..." }
egui = "0.31"

[target.'cfg(target_os = "android")'.dependencies]
android_logger = "0.14"
```

### 2. Импорты

```rust,ignore
use egui_android_framework::*;
use egui_android_framework::android::run;
use std::sync::mpsc;
```

### 3. Определите типы интентов и событий

```rust,ignore
enum Cmd { Incr }
enum Evt { Updated(u32) }
```

### 4. Реализуйте ViewModel

ViewModel хранит состояние, отправляет интенты в data layer, получает события:

```rust,ignore
struct MyVM { count: u32, cmd_tx: mpsc::Sender<Cmd> }

impl ViewModel for MyVM {
    type Intent = Cmd;
    type Event = Evt;

    fn create(ctx: ViewModelContext<Cmd, Evt>) -> Self {
        Self { count: 0, cmd_tx: ctx.command_tx().clone() }
    }

    fn handle(&mut self, cmd: Cmd) {
        let _ = self.cmd_tx.send(cmd);
    }

    fn on_event(&mut self, evt: Evt) {
        if let Evt::Updated(n) = evt { self.count = n; }
    }
}
```

### 5. Реализуйте Activity

Activity читает состояние из ViewModel и возвращает интенты:

```rust,ignore
impl Activity for MyActivity {
    type ViewModel = MyVM;
    type Application = MyApp;

    fn create(_: &AppContext<MyApp>) -> Self { Self }

    fn render(&mut self, ctx: &egui::Context, vm: &MyVM) -> Vec<Cmd> {
        let mut cmds = vec![];
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("+1").clicked() { cmds.push(Cmd::Incr); }
        });
        cmds
    }

    fn on_back_pressed(&mut self, _: &mut MyVM) -> bool { true }
}
```

### 6. Реализуйте Application

Application связывает Activity и ViewModel, запускает data layer:

```rust,ignore
impl Application for MyApp {
    type Activity = MyActivity;
    type ViewModel = MyVM;

    fn on_create(ctx: &mut AppContext<Self>) {
        ctx.config_mut().log_tag = "my-app".into();
    }

    fn create_view_model(ctx: &mut AppContext<Self>) -> MyVM {
        let vm_ctx = ctx.view_model_context();
        let (rx, tx) = ctx.take_data_layer_channels();
        // Запустите ваш data layer в отдельном потоке
        // std::thread::spawn(|| data_layer_worker(rx, tx));
        MyVM::create(vm_ctx)
    }

    fn create_activity(_: &mut AppContext<Self>) -> MyActivity { MyActivity }
}
```

### 7. Точка входа

```rust,ignore
#[no_mangle]
pub fn android_main(app: android_activity::AndroidApp) {
    run::<MyApp>(app);
}
```

### 8. Сборка и запуск

```bash
# Установите xbuild (cargo-apk)
cargo install xbuild

# Соберите и запустите на устройстве
cd your_project_path
x run --device adb:XXXXXXXX
```

Полный рабочий пример — [`examples/counter`](examples/counter).

## Архитектура

```
┌──────────┐   ┌──────────┐   ┌──────────────┐
│ Activity │ → │ ViewModel│   │  Data Layer  │
│ .render  │   │ .handle  │   │  cmd_rx      │
│ intents  │   │ .on_event│   │  evt_tx      │
└────┬─────┘   └────┬─────┘   └──────┬───────┘
     │               │               │
     ▼               ▼               ▼
┌────────────────────────────────────────────┐
│              egui-android-framework          │
│  ┌────────────┐  ┌──────────────────────┐  │
│  │ EGL backend │  │ run() — lifecycle   │  │
│  │ + input     │  │ + render + dispatch │  │
│  └────────────┘  └──────────────────────┘  │
└────────────────────────────────────────────┘
```

**Поток данных:**

```
UI (нажатие кнопки)
  → Activity::render() возвращает Vec<Cmd>
    → Фреймворк (run.rs) вызывает view_model.dispatch(cmd)
      → ViewModel::handle(cmd) — отправляет интент в data layer через self.cmd_tx
        → Data Layer (ваш поток) получает cmd из cmd_rx, обрабатывает
          → Data Layer шлёт Event через evt_tx.send(event)
            → Фреймворк (run.rs) вызывает vm_ctx.poll_events()
              → Фреймворк вызывает view_model.on_event(event)
                → ViewModel::on_event() обновляет self.state
                  → Следующий кадр: Activity::render() читает vm.state
```

## Тесты

20 тестов: каналы, dispatch, handle, on_event, AppConfig, LifecycleObserver.

## Зависимости

| Крейт | Назначение |
|---|---|
| `egui` 0.31 | GUI |
| `egui_glow` 0.31 | Рендеринг |
| `glow` 0.16 | OpenGL |
| `android-activity` 0.6 | NativeActivity |
| `ndk` 0.9 | NDK bindings |

## Лицензия

MIT или Apache 2.0
