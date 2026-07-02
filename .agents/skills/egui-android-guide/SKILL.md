---
name: egui-android-guide
description: Архитектура, правила и идиомы проекта egui-android-framework. Используй при доработке крейта, написании примеров, рефакторинге тестов.
---

# egui-android-framework: Архитектура и правила

Проект — MVI-фреймворк для запуска egui-приложений на Android,
разделённый на 7 крейтов в едином workspace.
Вдохновлён [Decompose](https://github.com/arkivanov/Decompose) и Jetpack Compose:
древовидная архитектура компонентов с однонаправленным потоком данных
и реактивным состоянием.

Сообщения отправляются из View в момент события через `Dispatcher`,
а не собираются в `Vec` и не возвращаются после рендера.

## Архитектура (MVI + Reactive State + Dispatcher)

```
                    ┌──────────────────────┐
                    │   framework          │  umbrella, re-exports
                    └──────┬───────────────┘
                           │
        ┌──────────────────┼──────────────────┐
        │                  │                  │
        ▼                  ▼                  ▼
┌──────────────┐   ┌──────────────┐   ┌──────────────┐
│  navigation  │   │     ui       │   │    core      │
│  ChildStack  │   │  remember    │   │  Component   │
│  Lifecycle   │   │  builders    │   │  ViewFn      │
│              │   │  modifier    │   │  Widget      │
└──────┬───────┘   └──────┬───────┘   │  Lifecycle   │
       │                  │           │  Ctx         │
       └──────────────────┼───────────┴──────┬───────┘
                          │                  │
                          ▼                  ▼
                   ┌──────────────────────────────┐
                   │         runtime              │
                   │  Application | Dispatcher    │
                   │  StateStore | UiNotifier     │
                   │  AppConfig | AppError        │
                   └──────────────┬───────────────┘
                                  │
                    ┌─────────────┴─────────────┐
                    │                           │
                    ▼                           ▼
          ┌─────────────────┐       ┌──────────────────────┐
          │    platform     │       │  platform-android    │
          │  Platform trait │◀──────│  EGL | NDK input    │
          │  PlatformEvent  │       │  run<A: Application> │
          │  FrameInput/Out │       │  egui_glow renderer  │
          │  PlatformConfig │       └──────────────────────┘
          └─────────────────┘
```

**Граф зависимостей (DAG, без циклов):**
```
platform-android → platform, runtime
runtime          → egui, tokio
core             → runtime
ui               → core
navigation       → core, ui
framework        → core, ui, navigation, runtime, platform, platform-android
```

## Поток данных (реактивный с Dispatcher)

```
UI (нажатие кнопки)
  → dispatch.dispatch(Msg::Increment)           ← в момент клика
    → Receiver (mpsc) накапливает сообщения
      ← после render: drain receiver
        → Component::handle(msg) — отправляет команду в data layer
          → Data Layer (фоновый поток) получает команду
            → store.update(|s| s.count += 1)    ← ЕДИНСТВЕННАЯ ТОЧКА
              → notify_tx.send(())               ← сигнал в Runtime
                → UiNotifier::check()
                  → ctx.request_repaint()
                  → waker.wake()
                    → poll_events() получает Wake
                      → frame() → render(state, &dispatcher) → новый UI
```

Никакого polling. Никаких `poll()`, `on_event()`, `needs_redraw`.
Состояние само уведомляет Runtime через `notify_tx`.

## Упрощённая MVI-модель

В проекте используется упрощённая модель, где **Intent и Message — это одно и то же**.

В классической MVI (как описано в `android-egui-architecture`):
```
Intent (сырое событие UI) → Message (семантическое действие) → Reducer
```

У нас сразу:
```
Message = и событие, и семантика
```

Пример:
```
Клик по кнопке "+" → dispatch(Msg::Increment) → handle(Msg::Increment) → reducer
```

Это осознанное упрощение для снижения бойлерплейта.
Архитектурный контракт (`android-egui-architecture`) описывает строгую модель с Intent,
но в коде проекта Intent и Message объединены.

Если проект вырастет и потребуется разделение (например, для валидации Intent
до превращения в Message) — введём отдельный тип Intent.

**Правило для агента:** не вводи разделение Intent/Message без явного указания.
Используй `Message` (или `Msg`) как единый тип для событий UI.

## Трейты и их расположение по крейтам

### `Application` — `egui-android-runtime`
- Корень DI. Владеет RootComponent, каналами, AppConfig.
- `create()` — создаёт приложение, каналы, запускает data layer.
- `root() / root_ref()` — доступ к корневому компоненту.
- `config() / config_mut()` — настройки (log_tag, target_fps).
- `create_notifier(ctx, wake) -> Option<UiNotifier>` — создаёт инфраструктуру уведомлений.
- `frame(ctx, raw_input) -> FullOutput` — один кадр: создаёт `Dispatcher`, вызывает `sync → render`, drain'ит receiver, обрабатывает сообщения через `handle()`.
- **Не наследует** `LifecycleObserver` (планируется, сейчас методы объявлены прямо в трейте).

### `Dispatcher<M>` — `egui-android-runtime`
- Абстракция над `std::sync::mpsc::Sender`. Скрывает канал от View.
- `new() -> (Self, Receiver<M>)` — создаёт пару dispatcher/receiver.
- `dispatch(msg)` — отправляет сообщение в момент события (клик, переключение и т.д.).
- `Clone` — можно передавать дочерним компонентам и в замыкания.
- View не знает про канал внутри — только про `dispatch()`.

### `Component` — `egui-android-core`
- Узел дерева навигации. Хранит snapshot состояния + ссылку на Store.
- `render(ui, &dispatcher)` — делегирует View-функции, сообщения диспатчатся через `Dispatcher`.
- `handle(msg)` — обрабатывает сообщение (отправляет команду в data layer).
- `sync_from_store()` — синхронизирует snapshot из Store (вызывается перед render).
- `state() -> &State` — текущий snapshot для View.
- Наследует `LifecycleObserver`.

### `StateStore<T>` — `egui-android-runtime`
- Реактивное состояние на `tokio::sync::watch`.
- `update(f)` — атомарно изменить состояние (+ уведомить подписчиков).
- `dispatch(msg, reducer)` — MVI-диспатч через reducer.
- `state() -> T` — snapshot текущего состояния.
- `subscribe() -> watch::Receiver<T>` — подписка на изменения.
- `clone_state() -> Self` — копия с разделяемым каналом.

### `UiNotifier` — `egui-android-runtime`
- Инфраструктурный уведомитель. Вызывается в главном потоке.
- `check()` — проверяет `mpsc::Receiver<()>` и при сигнале вызывает `repaint + wake`.
- Не знает про Domain, Components, Reducer.

### `Widget<M>` — `egui-android-core`
- Базовый трейт для всех виджетов и модификаторов.
- `render(&self, ui, dispatch)` — рендерит виджет, может диспатчить сообщения.

### `ModifierExt<M>` — `egui-android-ui`
- Extension trait для применения модификаторов (padding, size, background, align, clickable).
- Реализован blanket-impl для всех `Widget<M>`.

### `ChildStack<C, Comp>` — `egui-android-navigation`
- Стек дочерних компонентов с управлением жизненным циклом.
- `push / pop / replace / bring_to_front / clear`.
- Аналог `ChildStack` из Decompose.

### `LifecycleObserver` — `egui-android-core`
- Трейт с методами `on_create / on_start / on_resume / on_pause / on_stop / on_destroy`.
- Все методы имеют пустую реализацию по умолчанию.

## View — чистая функция с Dispatcher

```rust,ignore
type ViewFn<S, M> = fn(state: &S, ui: &mut egui::Ui, dispatch: &Dispatcher<M>);
```

View не хранит состояние, не знает о каналах, не имеет побочных эффектов.
Сообщения отправляются через `dispatch.dispatch(msg)` в момент события,
а не возвращаются списком после рендера.

Функция легко тестируется и переиспользуется.

### Пример View в новом стиле

```rust,ignore
fn counter_view(state: &u32, ui: &mut egui::Ui, dispatch: &Dispatcher<Msg>) {
    ui.label(format!("{}", state));
    if ui.button("+1").clicked() {
        dispatch.dispatch(Msg::Increment);
    }
}
```

## Каналы

| Канал | Тип | Откуда → Куда |
|---|---|---|
| `dispatcher` / `receiver` | `mpsc::channel::<Msg>()` | View → Component (через Dispatcher) |
| `cmd_tx` / `cmd_rx` | `mpsc::channel::<Msg>()` | Component::handle → Data Layer |
| `notify_tx` / `notify_rx` | `mpsc::channel::<()>()` | Data Layer → UiNotifier |
| `store` | `StateStore<T>` (watch) | Data Layer → Component (sync_from_store) |

Dispatcher создаётся каждый кадр в `frame()` и живёт один кадр.
Receiver drain'ится после render — все сообщения обрабатываются через `handle()`.

## Типовой frame() в Application

```rust,ignore
fn frame(&mut self, egui_ctx: &egui::Context, raw_input: egui::RawInput) -> egui::FullOutput {
    self.root.sync_from_store();

    let (dispatcher, receiver) = Dispatcher::new();

    let full_output = egui_ctx.run(raw_input, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.root.render(ui, &dispatcher);
        });
    });

    for msg in receiver.try_iter() {
        self.root.handle(msg);
    }

    full_output
}
```

## Android Lifecycle (как обрабатывается в platform-android)

| Событие | Действие |
|---|---|
| `InitWindow` | Пересоздать EGL surface, не трогать display/context/painter |
| `Resume` | `app_instance.on_resume()` |
| `Pause` | `app_instance.on_pause()` |
| `Stop` | `app_instance.on_stop()` — не чистить EGL |
| `Destroy` | `app_instance.on_destroy()`, destroy painter/EGL, `break` из цикла |

Lifecycle вызывается методами `Application` (пока не через `LifecycleObserver`).

## Структура проекта (workspace)

```
/
├── Cargo.toml              — workspace root
├── .agents/                — агентские скилы
│
├── crates/
│   ├── platform/           — egui-android-platform
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── platform.rs  — trait Platform
│   │       ├── event.rs     — PlatformEvent<W, I>
│   │       ├── frame.rs     — FrameInput, FrameOutput
│   │       └── config.rs    — PlatformConfig
│   │
│   ├── platform-android/   — egui-android-platform-android (cfg = android)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── run.rs       — run<A: Application>() — главный цикл
│   │       ├── egl_backend.rs — EGL FFI + EglState
│   │       └── input.rs     — InputState + process_input_events()
│   │
│   ├── runtime/            — egui-android-runtime
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── application.rs — trait Application + AppConfig + DataLayerHandle
│   │       ├── dispatcher.rs  — Dispatcher<M>
│   │       ├── store.rs       — StateStore<T> (+ 10 тестов)
│   │       ├── ui_notifier.rs — UiNotifier + AndroidWakeHandle
│   │       └── error.rs       — AppError
│   │
│   ├── core/               — egui-android-core
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── component.rs        — trait Component
│   │       ├── component_context.rs — ComponentContext + ComponentContextHandle
│   │       ├── lifecycle.rs        — LifecycleState + LifecycleObserver
│   │       ├── view.rs             — type ViewFn<S, M>
│   │       └── widget.rs           — trait Widget<M>
│   │
│   ├── ui/                 — egui-android-ui
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── remember.rs  — RememberState<T> + remember()
│   │       ├── builders.rs  — column(), row(), boxed()
│   │       └── modifier.rs  — Padded, SizedWidget, Background, Aligned, Clickable, ModifierExt
│   │
│   ├── navigation/         — egui-android-navigation
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       └── child_stack.rs — ChildStack<C, Comp> (+ 7 тестов)
│   │
│   └── framework/          — egui-android-framework (umbrella)
│       ├── Cargo.toml
│       └── src/
│           └── lib.rs — pub use всех крейтов
│
└── examples/
    ├── counter/            — реактивный счётчик
    │   ├── Cargo.toml
    │   └── src/
    │       ├── lib.rs
    │       ├── app.rs
    │       ├── component.rs
    │       ├── data_layer.rs
    │       ├── msg.rs
    │       └── view.rs
    │
    └── showcase/           — витрина всех возможностей
        ├── Cargo.toml
        └── src/
            ├── lib.rs
            ├── app.rs
            ├── navigation.rs
            ├── root_component.rs
            ├── components/
            └── screens/
```

## Зависимости (ключевые)

| Крейт | Где используется | Зачем |
|---|---|---|
| `egui` 0.31 | runtime, core, ui, platform | GUI-фреймворк |
| `egui_glow` 0.31 | platform-android | Рендеринг через OpenGL |
| `glow` 0.16 | platform-android | OpenGL context |
| `android-activity` 0.6 | platform-android | NativeActivity (только Android) |
| `ndk` 0.9 | platform-android | NDK bindings (NativeWindow) |
| `libc` 0.2 | platform-android | dlopen/dlsym для GL |
| `tokio` 1 (sync) | runtime | watch::channel для StateStore |
| `thiserror` 2.0 | runtime | AppError derive |
| `log` 0.4 | platform-android, runtime | Логирование |
| `android_logger` 0.14 | platform-android | Логирование в logcat |

Нет `crossbeam`, нет `OnceLock` — всё на стандартных `mpsc` + `tokio::sync::watch`.

## Правила взаимодействия с агентом

### Коммиты и push

Агент **НЕ** выполняет `git commit` или `git push` без явного разрешения пользователя.

Перед коммитом агент обязан:
1. Показать `git diff --stat` или summary изменений.
2. Показать предлагаемое сообщение коммита.
3. Спросить: «Коммит?» и дождаться подтверждения.

После коммита агент **НЕ** выполняет `git push` без отдельного разрешения.

## Важные правила

1. **Не использовать `OnceLock` / глобальные статические переменные** для хранения Sender'ов.
   Sender хранится в Application или Component, Dispatcher создаётся каждый кадр.
   — *Почему:* `OnceLock` не сбрасывается между запусками процесса.

2. **Не вызывать `std::process::exit(0)`** — фреймворк сам завершает цикл через `break`.

3. **EGL display/context переживает InitWindow** — при смене окна пересоздаётся
   только surface.

4. **Сообщения отправляются через `Dispatcher::dispatch()` в момент события**, а не возвращаются списком.
   View не возвращает `Vec<Message>`, а диспатчит их сразу через `&Dispatcher`.

5. **Все комментарии, логи и строки ошибок — на русском.**

6. **`store.update()` — единственная точка изменения состояния.** Никаких прямых
   присваиваний через `RwLock`.

7. **Data layer после `store.update()` всегда шлёт `notify_tx.send(())`.**

8. **Component синхронизируется из store через `sync_from_store()` в начале frame.**

9. **Главный цикл событийный:** `poll_events(16ms)` + `notifier.check()` + `frame()`.
   Никаких `poll()`, `on_event()`, `needs_redraw`.

10. **Упрощённая MVI-модель:** Intent и Message — это одно и то же (`Msg`).
    Не вводи разделение Intent/Message без явного указания.

11. **Импорты — через umbrella или напрямую:**
    ```rust
    // Через umbrella (если подключен egui-android-framework)
    use egui_android_framework::runtime::{Application, Dispatcher};
    use egui_android_framework::core::{Component, LifecycleObserver};

    // Напрямую (если подключен только конкретный крейт)
    use egui_android_runtime::Application;
    use egui_android_core::Component;
    ```

12. **При изменении кода проверять изоляцию крейтов:**
    - `runtime` не импортирует `core`, `ui`, `navigation`
    - `core` не импортирует `navigation`
    - `ui` не импортирует `navigation`
    - `platform` не импортирует `runtime`, `core`, `ui`, `navigation`

## Сборка и запуск

```bash
# Все крейты workspace
cargo check --workspace
cargo test --workspace

# Только конкретный крейт
cargo test -p egui-android-runtime
cargo test -p egui-android-navigation

# Пример counter (на хосте — только check, полный запуск — на Android)
cd examples/counter
x run --device adb:XXXXXXXX
```
