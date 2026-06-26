---
name: egui-android-guide
description: Архитектура, правила и идиомы проекта egui-android-framework. Используй при доработке крейта, написании примеров, рефакторинге тестов.
---

# egui-android-framework: Архитектура и правила

Этот крейт — MVI-фреймворк для запуска egui-приложений на Android.
Вдохновлён [Decompose](https://github.com/arkivanov/Decompose) и Jetpack Compose:
древовидная архитектура компонентов с однонаправленным потоком данных
и реактивным состоянием.

## Архитектура (MVI + Reactive State)

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
│  │  EGL backend  │  │  run() — lifecycle + input     │    │
│  │  + OpenGL ES  │  │  + frame() + UiNotifier        │    │
│  └──────────────┘  └────────────────────────────────┘    │
└──────────────────────────────────────────────────────────┘
```

**Поток данных (реактивный):**

```
UI (нажатие кнопки)
  → View(state, ui) возвращает Vec<Message>
    → Component::handle(msg) — отправляет команду в data layer
      → Data Layer (фоновый поток) получает команду
        → store.update(|s| s.count += 1)        ← ЕДИНСТВЕННАЯ ТОЧКА
          → notify_tx.send(())                   ← сигнал в Runtime
            → UiNotifier::check()
              → ctx.request_repaint()
              → waker.wake()
                → poll_events() получает Wake
                  → frame() → render(state) → новый UI
```

Никакого polling. Никаких `poll()`, `on_event()`, `needs_redraw`.
Состояние само уведомляет Runtime через `notify_tx`.

## Трейты и их назначение

### `Application`
- Корень DI. Владеет RootComponent, каналами, AppConfig.
- `create()` — создаёт приложение, каналы, запускает data layer.
- `root() / root_ref()` — доступ к корневому компоненту.
- `config() / config_mut()` — настройки (log_tag, target_fps).
- `create_notifier(ctx, wake) -> Option<UiNotifier>` — создаёт инфраструктуру уведомлений.
- `frame(ctx, raw_input) -> FullOutput` — один кадр: `sync → render → handle`.
- Наследует `LifecycleObserver` — lifecycle делегируется в RootComponent.

### `Component`
- Узел дерева навигации. Хранит snapshot состояния + ссылку на Store.
- `render(ui) -> Vec<Message>` — делегирует View-функции, возвращает сообщения.
- `handle(msg)` — логирует/обрабатывает сообщение.
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

## View — чистая функция

```rust,ignore
type ViewFn<S, M> = fn(state: &S, ui: &mut egui::Ui) -> Vec<M>;
```

View не хранит состояние, не знает о каналах, не имеет побочных эффектов.
Функция легко тестируется и переиспользуется.

## Каналы

| Канал | Тип | Откуда → Куда |
|-------|-----|---------------|
| `cmd_tx` / `cmd_rx` | `mpsc::channel::<Msg>()` | Component → Data Layer |
| `notify_tx` / `notify_rx` | `mpsc::channel::<()>()` | Data Layer → UiNotifier |
| `store` | `StateStore<T>` (watch) | Data Layer ↔ Component |

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
├── lib.rs              — экспорт публичных трейтов и модуля android
├── application.rs      — trait Application + AppConfig + AppState
├── component.rs        — trait Component
├── child_stack.rs      — ChildStack<C, Comp>
├── store.rs            — StateStore<T> (реактивное состояние)
├── ui_notifier.rs      — UiNotifier (инфраструктура)
├── component_context.rs
├── view.rs             — type ViewFn<S, M>
├── lifecycle.rs        — LifecycleState + LifecycleObserver
├── error.rs            — AppError
├── egui_subscriber.rs  — AndroidWakeHandle (устаревший, оставлен)
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
    └── ui_notifier_tests.rs  — 5 тестов UiNotifier
examples/
├── counter/            — старый пример (требует миграции)
└── counter2/           — реактивный счётчик (Application + Component + Store)
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

1. **Не использовать `OnceLock` / глобальные статические переменные`
1. **Не использовать `OnceLock` / глобальные статические переменные** для хранения Sender'ов.
   Sender хранится в Application или Component.
   — *Почему:* `OnceLock` не сбрасывается между запусками процесса.

2. **Не вызывать `std::process::exit(0)`** — фреймворк сам завершает цикл через `break`.

3. **EGL display/context переживает InitWindow** — при смене окна пересоздаётся
   только surface.

4. **Render возвращает `Vec<Message>`** — сообщения собираются в `render()`, потом
   фреймворк обрабатывает их через `handle()`.

5. **Все комментарии, логи и строки ошибок — на русском.**

6. **`store.update()` — единственная точка изменения состояния.** Никаких прямых
   присваиваний через `RwLock`.

7. **Data layer после `store.update()` всегда шлёт `notify_tx.send(())`.**

8. **Component синхронизируется из store через `sync_from_store()` в начале frame.**

9. **Главный цикл событийный:** `poll_events(16ms)` + `notifier.check()` + `frame()`.
   Никаких `poll()`, `on_event()`, `needs_redraw`.

## Сборка и запуск

```bash
cd examples/counter2
x run --device adb:XXXXXXXX

# Тесты (на хосте)
cargo test
# Все 55 тестов должны проходить
```
