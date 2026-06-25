---
name: egui-android-guide
description: Архитектура, правила и идиомы проекта egui-android-framework. Используй при доработке крейта, написании примеров, рефакторинге тестов.
---

# egui-android-framework: Архитектура и правила

Этот крейт — Decompose-style фреймворк для запуска egui-приложений на Android.
Вдохновлён [Decompose](https://github.com/arkivanov/Decompose): древовидная
архитектура компонентов с односторонним потоком данных.

## Архитектура (Decompose-style)

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
    → Component::handle(message) — отправляет команду в data layer
      → Data Layer (фоновый поток) получает команду, обрабатывает
        → Data Layer шлёт Event через evt_tx
          → Следующий кадр: Application::poll() читает evt_rx
            → Component::on_event() обновляет состояние
              → Следующий frame() рисует новое состояние
```

## Трейты и их назначение

### `Application`
- Корень DI. Владеет RootComponent, DataLayerHandle, AppConfig.
- `create()` — создаёт приложение, каналы, запускает data layer.
- `root() / root_ref()` — доступ к корневому компоненту.
- `config() / config_mut()` — настройки (log_tag, target_fps).
- `poll()` — опрос событий от data layer (вызывается перед каждым кадром).
- `frame(ctx, raw_input) -> FullOutput` — один кадр: рендеринг компонента
  через `root().render(ui)`, затем обработка сообщений через `root().handle()`.
- Наследует `LifecycleObserver` — lifecycle приложения делегируется в RootComponent.

### `Component`
- Узел дерева навигации. Владеет состоянием, получает сообщения от View.
- `render(ui) -> Vec<Message>` — делегирует View-функции, возвращает сообщения.
- `handle(msg)` — обрабатывает сообщение (отправляет в data layer, меняет состояние).
- `on_event(evt)` — получает событие от data layer (вызывается после `poll()`).
- `state() -> &State` — текущее состояние для View.
- Наследует `LifecycleObserver`.

### `LifecycleObserver`
- Опциональные колбэки: `on_create/start/resume/pause/stop/destroy`.
- Реализуется `Application` и `Component`.

### `ChildStack<C, Comp>`
- Стек дочерних компонентов с управлением жизненным циклом.
- `push(config, component)` — добавить на вершину, вызывается `on_create/start/resume`.
- `pop() -> Option<(C, Comp)>` — убрать с вершины, вызывается `on_pause/stop/destroy`.
- `replace(config, component)` — заменить верхний.
- `bring_to_front(config, component)` — переместить стек в указанное состояние.
- `clear()` — уничтожить все компоненты.
- `active() / active_mut() / active_config()` — доступ к текущему компоненту.
- Аналог `ChildStack` из Decompose.

### `ComponentContext<NavEvent, DataCmd, DataEvt>`
- Контекст, передаваемый в компонент при создании.
- `send_nav(event)` — отправить навигационное событие родителю.
- `send_cmd(cmd)` — отправить команду в data layer.
- `poll_events() -> Vec<DataEvt>` — забрать события от data layer.
- `data_cmd_tx() / nav_tx()` — клонировать Sender'ы для дочерних компонентов.
- Каналы через `std::sync::mpsc` (без `crossbeam`, без `OnceLock`).

## View — чистая функция

```rust,ignore
type ViewFn<S, M> = fn(state: &S, ui: &mut egui::Ui) -> Vec<M>;
```

View не хранит состояние, не знает о каналах, не имеет побочных эффектов.
Логика вся в `Component`. Функция легко тестируется и переиспользуется.

## Android Lifecycle (как обрабатывается в run.rs)

| Событие | Действие |
|---|---|
| `InitWindow` | Пересоздать EGL surface, не трогать display/context/painter |
| `Resume` | `app_instance.on_resume()`, `needs_redraw = true` |
| `Pause` | `app_instance.on_pause()` |
| `Stop` | `app_instance.on_stop()` — не чистить EGL |
| `Destroy` | `app_instance.on_destroy()`, destroy painter/EGL, `break` из цикла |
| `RedrawNeeded` | `needs_redraw = true` |

Lifecycle вызывается через `LifecycleObserver` на `Application`. Конкретный
Application должен делегировать вызовы в RootComponent (см. интеграционные тесты).

## Data Layer

- Фоновый поток с `mpsc`-каналами.
- Application создаёт каналы в `create()`, хранит `cmd_tx` и `evt_rx`.
- В `poll()` опрашивает `evt_rx`, обновляет состояние компонента.
- В `frame()` после рендера отправляет сообщения в data layer через `cmd_tx`.
- Component не знает о каналах — всё через Application.

## Структура проекта

```
src/
├── lib.rs              — экспорт публичных трейтов и модуля android
├── application.rs      — trait Application + AppConfig + AppState + DataLayerHandle
├── component.rs        — trait Component + DataEvent
├── component_context.rs — ComponentContext + ComponentContextHandle
├── child_stack.rs      — ChildStack<C, Comp> + LifecycleEvent
├── view.rs             — type ViewFn<S, M>
├── lifecycle.rs        — enum LifecycleState + trait LifecycleObserver
├── error.rs            — AppError
├── android/
│   ├── mod.rs          — реэкспорт run()
│   ├── egl_backend.rs  — EGL FFI bindings + EglState
│   ├── input.rs        — InputState + process_input_events()
│   └── run.rs          — run<A: Application>() — главный цикл
└── tests/
    ├── mod.rs
    ├── error_tests.rs
    ├── lifecycle_tests.rs
    └── integration_tests.rs  — 18 тестов (навигация, lifecycle, render→handle, интеграция)
examples/
├── counter/            — старый пример (не собирается, требует миграции)
└── counter2/           — рабочий пример счётчика (Application + Component + data layer)
```

## Зависимости (ключевые)

| Крейт | Зачем |
|---|---|
| `egui` 0.31 | GUI-фреймворк |
| `egui_glow` 0.31 | Рендеринг egui через OpenGL |
| `glow` 0.16 | OpenGL context |
| `android-activity` 0.6 | Android NativeActivity |
| `ndk` 0.9 | NDK bindings (NativeWindow) |
| `libc` 0.2 | dlopen/dlsym для загрузки GL функций |
| `thiserror` 2.0 | Ошибки (AppError) |
| `log` 0.4 + `android_logger` 0.14 | Логирование |

Нет `crossbeam`, нет `OnceLock` — всё на стандартных `mpsc` с `Arc<Mutex>`.

## Важные правила

1. **Не использовать `OnceLock` / глобальные статические переменные** для хранения Sender'ов.
   Sender хранится в Application или Component.
   — *Почему:* `OnceLock` не сбрасывается между запусками процесса. После Back →
   повторный запуск старый Sender закрыт, новый `set()` возвращает `Err`,
   сообщения уходят в никуда.

2. **Не вызывать `std::process::exit(0)`** — фреймворк сам завершает цикл через `break`.
   — *Почему:* `exit(0)` убивает процесс мгновенно, не дожидаясь других потоков
   (RenderThread, data layer). Это вызывает `pthread_mutex_lock called on a destroyed mutex`
   и SIGABRT.

3. **EGL display/context переживает InitWindow** — при смене окна пересоздаётся
   только surface, не context и не painter.
   — *Почему:* При сворачивании/разворачивании Android уничтожает и создаёт заново
   `ANativeWindow`. Если каждый раз уничтожать EGL display/context, теряются все
   загруженные текстуры и шрифты.

4. **`needs_redraw = true` на каждом кадре** — иначе UI не обновляется при событиях
   от data layer.
   — *Почему:* Если не форсировать `needs_redraw`, рендер происходит только по
   внешним событиям. События от `poll()` не триггерят системный redraw.

5. **Render возвращает `Vec<Message>`** — сообщения собираются в `render()`, потом
   фреймворк/Application обрабатывает их через `handle()`.
   — *Почему:* `render()` принимает `&Component` (иммутабельно), а `handle()` требует
   `&mut Component`. Вызов `handle()` внутри `egui_ctx.run()` создаёт пересечение
   заимствований. `Vec<Message>` разрывает последовательность: сначала читаем
   (`render`), потом пишем (`handle`).

6. **Все комментарии, логи и строки ошибок — на русском.**

## Планы (что ещё не реализовано)

| Возможность | Статус |
|---|---|
| Навигация: ChildStack + ComponentFactory | 🚧 В разработке |
| ComponentContext (передача зависимостей) | 🚧 В разработке |
| Сохранение состояния при повороте | 🔜 |
| InstanceState / Parcelable-аналог | 🔜 |
| BottomSheet / Modal / Dialog компоненты | 🔜 |
| Анимации переходов между экранами | 🔜 |
| snapshot-тесты View-функций | 🔜 |

## Сборка и запуск

```bash
# Рабочий пример
cd examples/counter2
x run --device adb:XXXXXXXX

# Тесты (на хосте)
cargo test

# Все 31 тест должны проходить
```
