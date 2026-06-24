# egui-android-framework

Легковесный фреймворк для запуска GUI-приложений на **egui** под Android.

## Возможности

- **EGL + OpenGL ES 2.0** — прямая работа с libEGL.so, без `glutin` и других тяжёлых зависимостей
- **MVVM-like архитектура** — `Application`, `Activity`, `ViewModel` с каналами для data layer
- **Android Lifecycle** — полная поддержка `Create → Start → Resume → Pause → Stop → Destroy`
- **Touch-ввод** — трансляция `MotionEvent` в `egui::Event`
- **Кнопка Back** — обрабатывается через `Activity::on_back_pressed()`
- **Смена конфигурации** — пересоздание EGL-поверхности при повороте/изменении окна
- **Минимальный размер APK** — `opt-level = "z"`, `lto = true`, `strip = true`

## Быстрый старт

Добавьте в `Cargo.toml`:

```toml
[dependencies]
egui-android-framework = { git = "..." }

# Обязательно для Android
[target.'cfg(target_os = "android")'.dependencies]
android_logger = "0.14"
```

Создайте точку входа (`src/lib.rs` или отдельный бинарник):

```rust,ignore
use egui_android_framework::{
    Activity, AppConfig, AppContext, Application, ViewModel, ViewModelContext,
};
use egui_android_framework::android::run;

// --- ViewModel ---
enum Cmd { Increment }
enum Evt {}

struct CounterVM { count: u32 }

impl ViewModel for CounterVM {
    type DataCommand = Cmd;
    type Event = Evt;

    fn create(_ctx: ViewModelContext<Cmd, Evt>) -> Self {
        Self { count: 0 }
    }

    fn handle(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Increment => self.count += 1,
        }
    }
}

// --- Activity ---
struct MainActivity;

impl Activity for MainActivity {
    type ViewModel = CounterVM;
    type Application = App;

    fn create(_ctx: &AppContext<App>) -> Self { Self }

    fn render(&mut self, ctx: &egui::Context, vm: &CounterVM) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(format!("Count: {}", vm.count));
            if ui.button("+1").clicked() {
                // TODO: отправить Cmd::Increment через data layer
            }
        });
    }
}

// --- Application ---
struct App;

impl Application for App {
    type Activity = MainActivity;
    type ViewModel = CounterVM;

    fn on_create(ctx: &mut AppContext<Self>) {
        ctx.config_mut().log_tag = "counter-app".into();
    }

    fn create_view_model(ctx: &mut AppContext<Self>) -> CounterVM {
        let vm_ctx = ctx.view_model_context();
        CounterVM::create(vm_ctx)
    }

    fn create_activity(_ctx: &mut AppContext<Self>) -> MainActivity {
        MainActivity
    }
}

// Точка входа для Android
#[no_mangle]
pub fn android_main(app: android_activity::AndroidApp) {
    run::<App>(app);
}
```

## Архитектура

```
┌─────────────────────────────────────────────────────────┐
│  Application (ваш крейт)                                │
│  ┌──────────┐  ┌──────────┐  ┌──────────────┐          │
│  │ Activity │  │ ViewModel│  │  Data Layer  │          │
│  │  .render │  │  .handle │  │  (ваш код)   │          │
│  └────┬─────┘  └────┬─────┘  └──────┬───────┘          │
│       │              │               │                  │
└───────┼──────────────┼───────────────┼──────────────────┘
        │              │               │
   ┌────▼──────────────▼───────────────▼────┐
   │         egui-android-framework          │
   │  ┌────────────┐  ┌──────────────────┐  │
   │  │  egl_backend│  │  run() — главный │  │
   │  │  (EGL FFI)  │  │  цикл + lifecycle│  │
   │  └────────────┘  │  + input + render │  │
   │                  └──────────────────┘  │
   │  ┌──────────────────────────────────┐  │
   │  │  input.rs — MotionEvent → egui   │  │
   │  └──────────────────────────────────┘  │
   └────────────────────────────────────────┘
                      │
          android-activity (NDK / NativeActivity)
                      │
                 Android / libEGL.so
```

Каналы между ViewModel и Data Layer:

- **ViewModel → Data Layer**: `Sender<DataCommand>` — команды (сохранить, загрузить, отправить)
- **Data Layer → ViewModel**: `Receiver<Event>` — события (данные загружены, ошибка)

## Зависимости

| Крейт | Версия | Назначение |
|---|---|---|
| `egui` | 0.31 | GUI-фреймворк |
| `egui_glow` | 0.31 | Рендеринг egui через OpenGL |
| `glow` | 0.16 | OpenGL context |
| `android-activity` | 0.6 | Android NativeActivity |
| `ndk` | 0.9 | NDK bindings (NativeWindow) |

## Лицензия

MIT или Apache 2.0
