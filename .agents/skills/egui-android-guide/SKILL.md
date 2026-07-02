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
│              │   │  widgets     │   │  Lifecycle   │
│              │   │  containers  │   │  Ctx         │
│              │   │  animation   │   └──────────────┘
│              │   │  theme       │
└──────┬───────┘   └──────┬───────┘
       │                  │
       └──────────────────┼──────────────────┐
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
- Extension trait для применения модификаторов: `padding()`, `size()`, `background()`, `align()`, `clickable()`, `clickable_with()`.
- Реализован blanket-impl для всех `Widget<M>`.

### `AnimationExt<M>` — `egui-android-ui`
- Extension trait для анимаций: `fade(opacity)`, `slide(direction, offset)`.
- Реализован blanket-impl для всех `Widget<M>`.

### `Button<M>`, `Text`, `Spacer`, `Icon` — `egui-android-ui`
- Готовые виджеты, реализующие `Widget<M>`.
- `Button::new(text).on_click(msg)` — при клике диспатчит сообщение (MVI-поток).
- `Button::new(text).on_click_with(closure)` — при клике вызывает closure (локальное UI-действие).
  Можно комбинировать с `on_click()` — сначала msg, потом closure.
- `Text::new(text)` — отображает строку.
- `Spacer::new(size)` — вертикальный отступ.
- `Icon::new(image)` — отображает изображение.

### `Column`, `Row`, `Stack`, `LazyColumn` — `egui-android-ui`
- Контейнеры на замыканиях (Compose-like), а не на builder pattern.
- Принимают `ui`, `dispatch` и closure, в котором рендерятся дочерние виджеты.
- `Column::new(ui, dispatch, |ui, dispatch| { ... })` — вертикальное расположение, spacing по умолч. 8.0.
- `Row::new(ui, dispatch, |ui, dispatch| { ... })` — горизонтальное расположение, spacing по умолч. 8.0.
- `Stack::new(ui, dispatch, |ui, dispatch| { ... })` — наложение виджетов.
- `LazyColumn::new(items, ui, dispatch, |item, ui, dispatch| { ... })` — скроллируемый список.
  Каждый элемент получает уникальный `ui.push_id(index)` для корректной работы `remember()`.

### `AnimatedVisibility<M>`, `Fade<W,M>`, `Slide<W,M>` — `egui-android-ui`
- Виджеты анимаций.
- `AnimatedVisibility::new(visible, duration).child(widget)` — плавное появление/исчезновение.
- `Fade::new(inner, opacity)` — прозрачность.
- `Slide::new(inner, direction, offset)` — смещение.

### `animate_value()`, `animate_bool()` — `egui-android-ui`
- Функции-хелперы интерполяции через `egui::Context`.

### `Theme`, `MaterialTheme`, `ColorPalette`, `Typography`, `Shapes` — `egui-android-ui`
- Система тем.
- `Theme::apply(ctx)`, `Theme::current(ctx)` — установка/чтение темы через `egui::Context::data()`.
- `MaterialTheme::light()` / `MaterialTheme::dark()` — Material Design 3 палитры.

### `remember()`, `RememberState<T>` — `egui-android-ui`
- Локальное UI-состояние между кадрами.
- `remember(ui, key, || init)` — создаёт/восстанавливает состояние.
- Внутри использует `Arc<RwLock<T>>` — методы `set()`, `modify()` принимают `&self` (не `&mut self`).
- Можно использовать внутри замыканий контейнеров (`Column::new`, `Row::new`, `LazyColumn::new`).
- `Clone` через `Arc::clone()` — все клоны разделяют одно состояние.
- `RememberState::get()` — возвращает `RwLockReadGuard<T>` (авторазыменование до `&T`).
- `RememberState::set(value)` — устанавливает новое значение.
- `RememberState::modify(|v| ...)` — изменяет значение через замыкание.

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

### Пример View в Compose-like стиле

```rust,ignore
fn counter_view(state: &u32, ui: &mut egui::Ui, dispatch: &Dispatcher<Msg>) {
    // remember внутри замыкания — работает благодаря Arc<RwLock<T>>
    Column::new(ui, dispatch, |ui, dispatch| {
        let show_details = remember(ui, "show_details", || false);

        Text::new("egui Counter").render(ui, dispatch);
        Spacer::new(16.0).render(ui, dispatch);

        Text::new(format!("{}", state))
            .padding(16.0)
            .background(egui::Color32::from_gray(40))
            .render(ui, dispatch);

        Button::new("+1")
            .on_click(Msg::Increment)
            .padding(8.0)
            .background(egui::Color32::from_rgb(0, 128, 255))
            .render(ui, dispatch);

        // Локальное UI-действие через on_click_with
        Button::new(if *show_details.get() { "Скрыть" } else { "Показать" })
            .on_click_with({
                let show_details = show_details.clone();
                move |_ui, _dispatch| show_details.modify(|v| *v = !*v)
            })
            .render(ui, dispatch);

        AnimatedVisibility::new(*show_details.get(), 0.3)
            .child(Text::new("Анимированный текст").padding(12.0))
            .render(ui, dispatch);
    });
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
│   │   └── src/
│   │       ├── platform.rs  — trait Platform
│   │       ├── event.rs     — PlatformEvent<W, I>
│   │       ├── frame.rs     — FrameInput, FrameOutput
│   │       └── config.rs    — PlatformConfig
│   │
│   ├── platform-android/   — egui-android-platform-android (cfg = android)
│   │   └── src/
│   │       ├── run.rs       — run<A: Application>() — главный цикл
│   │       ├── egl_backend.rs — EGL FFI + EglState
│   │       └── input.rs     — InputState + process_input_events()
│   │
│   ├── runtime/            — egui-android-runtime
│   │   └── src/
│   │       ├── application.rs — trait Application + AppConfig + DataLayerHandle
│   │       ├── dispatcher.rs  — Dispatcher<M>
│   │       ├── store.rs       — StateStore<T>
│   │       ├── ui_notifier.rs — UiNotifier + AndroidWakeHandle
│   │       └── error.rs       — AppError
│   │
│   ├── core/               — egui-android-core
│   │   └── src/
│   │       ├── component.rs        — trait Component
│   │       ├── component_context.rs — ComponentContext
│   │       ├── lifecycle.rs        — LifecycleState + LifecycleObserver
│   │       ├── view.rs             — type ViewFn<S, M>
│   │       └── widget.rs           — trait Widget<M>
│   │
│   ├── ui/                 — egui-android-ui
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── remember.rs  — RememberState + remember()
│   │       ├── builders.rs  — column(), row(), boxed()
│   │       ├── modifier.rs  — Padded, SizedWidget, Background, Aligned, Clickable, ModifierExt
│   │       ├── widgets/     — Button, Text, Spacer, Icon
│   │       ├── containers/  — Column, Row, Stack, LazyColumn
│   │       ├── animation/   — AnimatedVisibility, Fade, Slide, AnimationExt, animate_*
│   │       └── theme/       — Theme, ColorPalette, MaterialTheme, Typography, Shapes
│   │
│   ├── navigation/         — egui-android-navigation
│   │   └── src/
│   │       └── child_stack.rs — ChildStack<C, Comp>
│   │
│   └── framework/          — egui-android-framework (umbrella)
│       └── src/
│           └── lib.rs — pub use всех крейтов
│
└── examples/
    ├── counter/            — реактивный счётчик (Compose-like view)
    └── showcase/           — витрина с навигацией (7 экранов)
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

13. **Compose-like UI строится через замыкания:**
    ```rust
    use egui_android_framework::ui::{
        widgets::{Button, Text, Spacer, Widget},
        containers::Column,
        modifier::ModifierExt,
        remember,
    };

    // Контейнеры принимают ui, dispatch и closure
    Column::new(ui, dispatch, |ui, dispatch| {
        Text::new("Заголовок").render(ui, dispatch);

        // remember внутри замыкания
        let count = remember(ui, "counter", || 0i32);
        Text::new(format!("{}", count.get())).render(ui, dispatch);

        // on_click — MVI-поток
        Button::new("Кнопка").on_click(Msg::Clicked).render(ui, dispatch);

        // on_click_with — локальное UI-действие
        Button::new("+1").on_click_with({
            let count = count.clone();
            move |_ui, _dispatch| count.modify(|c| *c += 1)
        }).render(ui, dispatch);
    });
    ```

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
