---
name: egui-android-guide
description: Архитектура, правила и идиомы проекта egui-android-framework. Используй при доработке крейта, написании примеров, рефакторинге тестов.
---

# egui-android-framework: Архитектура и правила

Этот крейт — MVI-фреймворк для запуска egui-приложений на Android.
Вдохновлён [Decompose](https://github.com/arkivanov/Decompose) и Jetpack Compose:
древовидная архитектура компонентов с однонаправленным потоком данных
и реактивным состоянием.

Сообщения отправляются из View в момент события через `Dispatcher`,
а не собираются в `Vec` и не возвращаются после рендера.

## Архитектура (MVI + Reactive State + Dispatcher)

```
┌──────────────┐      ┌──────────────┐      ┌──────────────┐
│  Application  │      │  Component   │      │  Data Layer  │
│  .frame()     │      │  .render()   │      │  store.rw    │
│  .notify()    │  →   │  .handle()   │  ↔   │  notify_tx   │
│  .root()      │      │  .state()    │      └──────┬───────┘
└──────┬───────┘      └──────┬───────┘             │
       │                     │                      │
       ▼                     ▼                      ▼
┌──────────────────────────────────────────────────────────┐
│              egui-android-framework                        │
│  ┌──────────────┐  ┌────────────────────────────────┐    │
│  │  Dispatcher   │  │  run() — lifecycle + input     │    │
│  │  .dispatch()  │  │  + frame() + UiNotifier        │    │
│  └──────┬───────┘  └────────────────────────────────┘    │
│         │                                                 │
│         └── Сообщение отправляется в момент события       │
└──────────────────────────────────────────────────────────┘
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

## Трейты и их назначение

### `Application`
- Корень DI. Владеет RootComponent, каналами, AppConfig.
- `create()` — создаёт приложение, каналы, запускает data layer.
- `root() / root_ref()` — доступ к корневому компоненту.
- `config() / config_mut()` — настройки (log_tag, target_fps).
- `create_notifier(ctx, wake) -> Option<UiNotifier>` — создаёт инфраструктуру уведомлений.
- `frame(ctx, raw_input) -> FullOutput` — один кадр: создаёт `Dispatcher`, вызывает `sync → render`, drain'ит receiver, обрабатывает сообщения через `handle()`.
- Наследует `LifecycleObserver` — lifecycle делегируется в RootComponent.

### `Dispatcher<M>`
- Абстракция над `std::sync::mpsc::Sender`. Скрывает канал от View.
- `new() -> (Self, Receiver<M>)` — создаёт пару dispatcher/receiver.
- `dispatch(msg)` — отправляет сообщение в момент события (клик, переключение и т.д.).
- `Clone` — можно передавать дочерним компонентам и в замыкания.
- View не знает про канал внутри — только про `dispatch()`.

### `Component`
- Узел дерева навигации. Хранит snapshot состояния + ссылку на Store.
- `render(ui, &dispatcher)` — делегирует View-функции, сообщения диспатчатся через `Dispatcher`.
- `handle(msg)` — обрабатывает сообщение (отправляет команду в data layer).
- `sync_from_store()` — синхронизирует snapshot из Store (вызывается перед render).
- `state() -> &State` — текущий snapshot для View.
- Наследует `LifecycleObserver`.

### `StateStore<T>`
- Реактивное состояние на `tokio::sync::watch`.
- `update(f)` — атомарно изменить состояние (+ уведомить подписчиков).
- `dispatch(msg, reducer)` — MVI-диспатч через reducer.
- `state() -> T` — snapshot текущего состояния.
- `subscribe() -> watch::Receiver<T>` — подписка на изменения.
- `clone_state() -> Self` — копия с разделяемым каналом.

### `UiNotifier`
- Инфраструктурный уведомитель. Вызывается в главном потоке.
- `check()` — проверяет `mpsc::Receiver<()>` и при сигнале вызывает `repaint + wake`.
- Не знает про Domain, Components, Reducer.

### `ChildStack<C, Comp>`
- Стек дочерних компонентов с управлением жизненным циклом.
- `push / pop / replace / bring_to_front / clear`.
- Аналог `ChildStack` из Decompose.

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
| `store` | `StateStore<T>` (watch) | Data Layer ↔ Component |

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

## Android Lifecycle (как обрабатывается в run.rs)

| Событие | Действие |
|---|---|
| `InitWindow` | Пересоздать EGL surface, не трогать display/context/painter |
| `Resume` | `app_instance.on_resume()` |
| `Pause` | `app_instance.on_pause()` |
| `Stop` | `app_instance.on_stop()` — не чистить EGL |
| `Destroy` | `app_instance.on_destroy()`, destroy painter/EGL, `break` из цикла |

Lifecycle вызывается через `LifecycleObserver` на `Application`.

## Структура проекта

```
src/
├── lib.rs              — экспорт публичных трейтов и модулей
├── application.rs      — trait Application + AppConfig + AppState
├── component.rs        — trait Component
├── component_context.rs — контекст компонента (dispatcher, store, lifecycle)
├── child_stack.rs      — ChildStack<C, Comp>
├── dispatcher.rs       — Dispatcher<M> (отправка сообщений из View)
├── store.rs            — StateStore<T> (реактивное состояние)
├── ui_notifier.rs      — UiNotifier + AndroidWakeHandle
├── view.rs             — type ViewFn<S, M> (с &Dispatcher<M>)
├── lifecycle.rs        — LifecycleState + LifecycleObserver
├── error.rs            — AppError
├── android/
│   ├── mod.rs          — реэкспорт run()
│   ├── egl_backend.rs  — EGL FFI bindings + EglState
│   ├── input.rs        — InputState + process_input_events()
│   └── run.rs          — run<A: Application>() — событийный главный цикл
└── tests/
    ├── mod.rs
    ├── error_tests.rs
    ├── lifecycle_tests.rs
    ├── integration_tests.rs  — 22 теста (MVI, store, component, navigation)
    └── ui_notifier_tests.rs  — 6 тестов (UiNotifier + AndroidWakeHandle)
examples/
└── counter_example/    — реактивный счётчик (Application + Component + Store + Dispatcher)
```

## Зависимости (ключевые)

| Крейт | Зачем |
|---|---|
| `egui` 0.31 | GUI-фреймворк |
| `egui_glow` 0.31 | Рендеринг через OpenGL |
| `glow` 0.16 | OpenGL context |
| `android-activity` 0.6 | NativeActivity |
| `ndk` 0.9 | NDK bindings (NativeWindow) |
| `libc` 0.2 | dlopen/dlsym для GL |
| `tokio` 1 (sync) | watch::channel для StateStore |
| `log` 0.4 + `android_logger` 0.14 | Логирование |

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

## Сборка и запуск

```bash
cd examples/counter_example
x run --device adb:XXXXXXXX

# Тесты (на хосте)
cargo test
# Все 51 тест должен проходить
```
