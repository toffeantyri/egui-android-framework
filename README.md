# egui-android-framework

Легковесный фреймворк для запуска GUI-приложений на **egui** под Android.

Вдохновлён [Decompose](https://github.com/arkivanov/Decompose) — древовидная
архитектура компонентов с односторонним потоком данных.

## Возможности

- **EGL + OpenGL ES 2.0** — прямая работа с libEGL.so, без `glutin`
- **Decompose-style архитектура** — `Application`, `Component` + чистая View-функция,
  `ChildStack` для навигации
- **Android Lifecycle** — полная поддержка `Create → Start → Resume → Pause → Stop → Destroy`
- **Touch-ввод** — трансляция `MotionEvent` в `egui::Event`
- **Кнопка Back** — обрабатывается через компонент
- **Смена конфигурации** — пересоздание EGL-поверхности при повороте/изменении окна
- **Data layer** — фоновый поток с mpsc-каналами (отдельная бизнес-логика)
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
use egui_android_framework::{AppConfig, Application, Component, LifecycleObserver};
use egui_android_framework::android::run;
use std::sync::mpsc;
```

### 3. Определите сообщения и события

```rust,ignore
enum Msg { Increment }
enum Evt { CountUpdated(u32) }
```

### 4. Реализуйте Component

Компонент владеет состоянием и делегирует отрисовку View-функции:

```rust,ignore
struct CounterComponent { count: u32, cmd_tx: mpsc::Sender<Msg> }

impl LifecycleObserver for CounterComponent {}

impl Component for CounterComponent {
    type State = u32;
    type Message = Msg;

    fn render(&self, ui: &mut egui::Ui) -> Vec<Self::Message> {
        counter_view(&self.count, ui)
    }

    fn handle(&mut self, msg: Self::Message) {
        if let Msg::Increment = msg {
            let _ = self.cmd_tx.send(Msg::Increment);
        }
    }

    fn state(&self) -> &Self::State { &self.count }
}
```

### 5. Реализуйте View (чистая функция)

```rust,ignore
fn counter_view(state: &u32, ui: &mut egui::Ui) -> Vec<Msg> {
    let mut messages = vec![];
    egui::CentralPanel::default().show(ui.ctx(), |ui| {
        if ui.button("+1").clicked() { messages.push(Msg::Increment); }
    });
    messages
}
```

### 6. Реализуйте Application

Application — корень DI. Создаёт каналы, запускает data layer,
владеет корневым компонентом:

```rust,ignore
struct CounterApp {
    root: CounterComponent,
    config: AppConfig,
    cmd_tx: mpsc::Sender<Msg>,
    evt_rx: mpsc::Receiver<Evt>,
}

impl LifecycleObserver for CounterApp {}

impl Application for CounterApp {
    type RootComponent = CounterComponent;

    fn create() -> Self { /* создание каналов, запуск data layer */ }
    fn root(&mut self) -> &mut CounterComponent { &mut self.root }
    fn root_ref(&self) -> &CounterComponent { &self.root }
    fn config(&self) -> &AppConfig { &self.config }
    fn config_mut(&mut self) -> &mut AppConfig { &mut self.config }

    fn frame(&mut self, egui_ctx: &egui::Context, raw_input: egui::RawInput) -> egui::FullOutput {
        // 1. Опрос событий от data layer
        while let Ok(evt) = self.evt_rx.try_recv() { /* обновить root */ }
        // 2. Рендеринг
        let mut messages = vec![];
        let output = egui_ctx.run(raw_input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                messages = self.root.render(ui);
            });
        });
        // 3. Обработка сообщений
        for msg in messages { self.root.handle(msg.clone()); let _ = self.cmd_tx.send(msg); }
        output
    }
}
```

### 7. Точка входа

```rust,ignore
#[no_mangle]
pub fn android_main(app: android_activity::AndroidApp) {
    run::<CounterApp>(app);
}
```

### 8. Сборка и запуск

```bash
cargo install xbuild
cd your_project_path
x run --device adb:XXXXXXXX
```

Полный рабочий пример — [`examples/counter2`](examples/counter2).

## Архитектура

```
┌──────────────┐      ┌──────────────┐      ┌──────────────┐
│  Application  │      │  Component   │      │  Data Layer  │
│  .frame()     │      │  .render()   │      │  cmd_rx      │
│  .poll()      │  →   │  .handle()   │  ↔   │  evt_tx      │
│  .root()      │      │  .state()    │      └──────┬───────┘
└──────┬───────┘      └──────┬───────┘             │
       │                     │                      │
       ▼                     ▼                      ▼
┌──────────────────────────────────────────────────────────┐
│              egui-android-framework                        │
│  ┌──────────────┐  ┌────────────────────────────────┐    │
│  │  EGL backend  │  │  run() — lifecycle + input     │    │
│  │  + OpenGL ES  │  │  + frame() + EGL swap          │    │
│  └──────────────┘  └────────────────────────────────┘    │
└──────────────────────────────────────────────────────────┘
```

**Поток данных:**

```
UI (нажатие кнопки)
  → View(state, ui) возвращает Vec<Message>
    → Component::handle(message) — опционально отправляет в data layer
      → Data Layer (фоновый поток) получает команду, обрабатывает
        → Data Layer шлёт Event через evt_tx
          → Следующий кадр: Application::poll() читает evt_rx
            → Component::on_event() обновляет состояние
              → Следующий render() рисует новое состояние
```

## Примеры

- [`examples/counter2`](examples/counter2) — счётчик с фоновым data layer,
  демонстрирует `Component`, `Application`, View-функцию, `run()`.

## Планы (Decompose-style)

| Возможность | Статус |
|---|---|
| Component + ViewFn | ✅ Реализовано |
| ChildStack (push/pop/replace) | ✅ Реализовано |
| Lifecycle компонентов (onCreate/onDestroy) | ✅ Реализовано (через LifecycleObserver) |
| Application как корень DI | ✅ Реализовано |
| run() — главный цикл | ✅ Реализовано |
| Навигация: ChildStack + ComponentFactory | 🚧 В разработке |
| Сохранение состояния при повороте | 🔜 |
| InstanceState / Parcelable-аналог | 🔜 |
| BottomSheet / Modal / Dialog компоненты | 🔜 |
| Анимации переходов между экранами | 🔜 |
| Тестирование: snapshot-тесты View-функций | 🔜 |
| ComponentContext (навигация + DI) | 🚧 В разработке |

## Тесты

31 тест: ChildStack (навигация), жизненный цикл компонентов, Component (render→handle),
интеграция Application + Component + ChildStack, LifecycleObserver, AppConfig, AppError.

## Зависимости

| Крейт | Назначение |
|---|---|
| `egui` 0.31 | GUI |
| `egui_glow` 0.31 | Рендеринг через OpenGL |
| `glow` 0.16 | OpenGL context |
| `android-activity` 0.6 | NativeActivity |
| `ndk` 0.9 | NDK bindings (NativeWindow) |
| `libc` 0.2 | dlopen/dlsym для загрузки GL функций |
| `log` 0.4 + `android_logger` 0.14 | Логирование |

## Лицензия

MIT или Apache 2.0
