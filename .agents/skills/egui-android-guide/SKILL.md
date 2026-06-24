---
name: egui-android-guide
description: Архитектура, правила и идиомы проекта egui-android-framework. Используй при доработке крейта, написании примеров, рефакторинге тестов.
---

# egui-android-framework: Архитектура и правила

Этот крейт — легковесный MVVM-фреймворк для запуска egui-приложений на Android.

## Архитектура (MVVM)

```
┌──────────┐     ┌──────────┐     ┌──────────────┐
│ Activity │  →  │ ViewModel│     │  Data Layer  │
│ .render  │     │ .handle  │     │  cmd_rx      │
│ intents  │     │ .on_event│     │  evt_tx      │
└────┬─────┘     └────┬─────┘     └──────┬───────┘
     │                │                  │
     └────────────────┼──────────────────┘
                      ▼
         ┌────────────────────────┐
         │  egui-android-framework│
         │  run() — lifecycle     │
         │  + input + render     │
         │  + dispatch интентов   │
         └────────────────────────┘
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

## Трейты и их назначение

### `Application`
- Точка сборки приложения.
- Ассоциирует `Activity` и `ViewModel`.
- `on_create(ctx)` — настройка `AppConfig` (log_tag, target_fps).
- `create_view_model(ctx)` — создаёт каналы (`ctx.view_model_context()`), забирает их (`take_data_layer_channels()`), запускает data layer, возвращает ViewModel.
- `create_activity(ctx)` — создаёт Activity.

### `Activity`
- UI-слой. Читает `&ViewModel`, возвращает `Vec<Intent>`.
- `render(ctx, vm) → Vec<Cmd>` — отрисовка, интенты возвращаются, фреймворк диспатчит их.
- `on_back_pressed(vm) → bool` — true = выйти из приложения.
- Не хранит Sender'ы, не знает о каналах.

### `ViewModel`
- Владеет состоянием и `Sender<Intent>` (получен из `ViewModelContext`).
- `handle(cmd)` — обрабатывает интент от UI, отправляет в data layer через `self.cmd_tx`.
- `on_event(evt)` — получает события из data layer, обновляет состояние.
- `ViewModelContext` содержит `command_tx()` для клонирования Sender.

### `LifecycleObserver`
- Опциональные колбэки: `on_create/start/resume/pause/stop/destroy`.

## Android Lifecycle (как обрабатывается в run.rs)

| Событие | Действие |
|---|---|
| `InitWindow` | Пересоздать EGL surface, не трогать display/context/painter |
| `Resume` | `activity.on_resume()`, `needs_redraw = true` |
| `Pause` | `activity.on_pause()` |
| `Stop` | `activity.on_stop()` — не чистить EGL |
| `Destroy` | `activity.on_destroy()`, destroy painter/EGL, `break` из цикла |
| `RedrawNeeded` | `needs_redraw = true` |

## Интенты и события

- `Intent` — интент от UI к data layer (enum).
- `Event` — событие от data layer к ViewModel (enum).
- Каналы создаются в `create_view_model()` через `ctx.view_model_context()`.
- Data layer получает `(Receiver<Cmd>, Sender<Evt>)` через `take_data_layer_channels()`.
- Sender для интентов хранится в ViewModel, клонируется из `ctx.command_tx()`.

## Структура проекта

```
src/
├── lib.rs              — экспорт публичных трейтов и модуля android
├── activity.rs         — trait Activity
├── application.rs      — trait Application + AppContext + AppConfig
├── lifecycle.rs        — enum LifecycleState + trait LifecycleObserver
├── view_model.rs       — trait ViewModel + ViewModelContext
├── error.rs            — AppError
├── android/
│   ├── mod.rs          — реэкспорт run()
│   ├── egl_backend.rs  — EGL FFI bindings + EglState
│   ├── input.rs        — InputState + process_input_events()
│   └── run.rs          — run<A>() — главный цикл
└── tests/              — 20+ тестов
examples/
└── counter/            — рабочий пример счётчика
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
| `raw-window-handle` 0.6 | Получение ANativeWindow* |

## Важные правила

1. **Не использовать `OnceLock` / глобальные статические переменные** для хранения Sender'ов. Sender хранится в самой ViewModel.
   — *Почему:* `OnceLock` не сбрасывается между запусками процесса. После Back → повторный запуск старый Sender закрыт, новый `set()` возвращает `Err`, интенты уходят в никуда. Sender, сохранённый в полях ViewModel, создаётся заново при каждом создании процесса.

2. **Не вызывать `std::process::exit(0)`** в колбэках `on_back_pressed` или render — фреймворк сам завершает цикл через `break`.
   — *Почему:* `exit(0)` убивает процесс мгновенно, не дожидаясь других потоков (RenderThread, data layer). Это вызывает `pthread_mutex_lock called on a destroyed mutex` и SIGABRT. `break` из цикла позволяет `run()` вернуться, и android-activity корректно завершает NativeActivity.

3. **EGL display/context переживает InitWindow** — при смене окна пересоздаётся только surface, не context и не painter.
   — *Почему:* При сворачивании/разворачивании Android уничтожает и создаёт заново `ANativeWindow`. Если каждый раз уничтожать EGL display/context, теряются все загруженные текстуры и шрифты. Пересоздание только surface сохраняет контекст, и `egui_glow::Painter` остаётся валидным.

4. **`needs_redraw = true` на каждом кадре** — иначе UI не обновляется при событиях от data layer.
   — *Почему:* Если не форсировать `needs_redraw`, рендер происходит только по внешним событиям (нажатие, RedrawNeeded). События от `poll_events()` не триггерят системный redraw, поэтому изменения состояния ViewModel не отобразятся до следующего touch-события.

5. **Render возвращает `Vec<Intent>`** — интенты собираются во время отрисовки, потом фреймворк диспатчит их в ViewModel.
   — *Почему:* `render()` принимает `&ViewModel` (иммутабельно), а `dispatch()` требует `&mut ViewModel`. Вызов `dispatch()` внутри `egui_ctx.run()` создаёт пересечение заимствований. `Vec<Cmd>` разрывает последовательность: сначала читаем (`render`), потом пишем (`dispatch`).

6. **Все комментарии, логи и строки ошибок — на русском.**

## Сборка и запуск

```bash
# Сборка под Android через xbuild
cd examples/counter
x run --device adb:XXXXXXXX

# Тесты (на хосте)
cargo test
```
