[@android-egui-architecture](file:///C:/Users/Zavod9/StudioProjects/egui-android-framework/.agents/skills/android-egui-architecture/) [@task-evaluation-short](file:///C:/Users/Zavod9/StudioProjects/egui-android-framework/.agents/skills/task-evaluation-short/)  # Push vs Polling для event-driven UI

## Короткий ответ

Да, имеет смысл. Более того — **большая часть push-инфраструктуры уже есть**, нужно только перестать крутить цикл вхолостую.

## Что уже есть (push-механизмы)

```
StateStore::update()
  └─ watch::send_if_modified()          ← PUSH: подписчики получают уведомление

Waker::wake()
  └─ run_on_java_main_thread()          ← PUSH: будит Android main looper

egui::Context::request_repaint()        ← PUSH: egui сам знает, когда нужен кадр

full_output.viewport_output.get(&ViewportId::ROOT).map(|v| v.repaint_delay).unwrap_or(Duration::ZERO)  ← PUSH: egui говорит "следующий кадр через N"

> **Важно:** `repaint_delay` находится не в `PlatformOutput`, а в `ViewportOutput` — поле `full_output.viewport_output[ViewportId::ROOT].repaint_delay`. Тип — `std::time::Duration`.
```

## Что сейчас ломает push-модель

Одна строка в `gl_backend.rs` / `native_backend.rs`:

```rust
// drain_lifecycle_events()
app.poll_events(
    Some(Duration::from_millis(0)),   // ← NON-BLOCKING: цикл крутится постоянно
    |event| ...
);
```

Из-за `timeout = 0` цикл `while state.tick(...)` в `run.rs` **никогда не блокирует**. Каждый проход:

```
poll_events(0ms)  →  мгновенно возвращает []
rt_ctx.check()    →  try_recv() → ничего
frame()           →  рендер вхолостую
swap_buffers()    →  GPU рисует тот же кадр
→ повторить ~60 раз в секунду, даже если ничего не изменилось
```

## Как выглядит push-модель

```
                    ┌─────────────────────────────────┐
                    │  poll_events(timeout)            │
                    │  БЛОКИРУЕТ до:                   │
                    │  • touch / lifecycle             │
                    │  • Waker (run_on_java_main)      │
                    │  • repaint_delay (анимации egui) │
                    └──────────┬──────────────────────┘
                               │ проснулись
                               ▼
                    ┌─────────────────────────────────┐
                    │  rt_ctx.check()                  │
                    │  try_recv() — ОК, вызывается     │
                    │  только по событию, не polling   │
                    └──────────┬──────────────────────┘
                               ▼
                    ┌─────────────────────────────────┐
                    │  frame() → run_ui()              │
                    │  Рендер кадра                    │
                    └──────────┬──────────────────────┘
                               ▼
                    ┌─────────────────────────────────┐
                    │  repaint_delay = output          │
                    │  .viewport_output                │
                    │  [ViewportId::ROOT].repaint_delay│
                    └──────────┬──────────────────────┘
                               ▼
                    ┌─────────────────────────────────┐
                    │  swap_buffers                    │
                    └──────────┬──────────────────────┘
                               ▼
                    ┌─────────────────────────────────┐
                    │  poll_events(repaint_delay)      │
                    │  Снова блокирует...              │
                    │  CPU спит. Батарея цела.         │
                    └─────────────────────────────────┘
```

## Конкретные изменения (4 файла, ~20 строк)

### 1. `backend/mod.rs` — trait получает timeout

```rust
pub trait AndroidBackend {
    // Было:
    // fn poll_events(&mut self) -> Vec<BackendEvent>;

    // Стало:
    fn poll_events(&mut self, timeout: Option<Duration>) -> Vec<BackendEvent>;
    // ...
}
```

### 2. `gl_backend.rs` / `native_backend.rs` — передаём timeout

```rust
fn drain_lifecycle_events(&mut self, timeout: Option<Duration>) {
    let events = &mut self.events;
    let app = &self.app;
    app.poll_events(timeout, |event| match event {  // ← было Some(0ms)
        // ... без изменений
    });
}
```

### 3. `loop.rs` — `RunState` хранит `repaint_delay`

```rust
pub struct RunState {
    // ... существующие поля ...
    repaint_delay: Duration,   // ← НОВОЕ
}

impl RunState {
    pub fn new() -> Self {
        Self {
            // ...
            repaint_delay: Duration::ZERO,  // первый кадр — сразу
        }
    }

    pub fn tick<A: Application>(/* ... */) -> bool {
        // Было:
        // let backend_events = backend.poll_events();

        // Стало:
        let timeout = if self.repaint_delay == Duration::ZERO {
            Some(Duration::ZERO)       // первый кадр / срочный repaint
        } else if self.repaint_delay >= Duration::from_secs(3600) {
            None                        // блокировать до события
        } else {
            Some(self.repaint_delay)   // ждать (анимации egui)
        };
        let backend_events = backend.poll_events(timeout);

        // ... обработка событий, frame() — без изменений ...

        let full_output = app_instance.frame(egui_ctx, raw_input);

        // ← НОВОЕ: запоминаем, когда egui хочет следующий кадр
        // repaint_delay — в ViewportOutput, не в PlatformOutput!
        self.repaint_delay = full_output
            .viewport_output
            .get(&egui::ViewportId::ROOT)
            .map(|v| v.repaint_delay)
            .unwrap_or(Duration::ZERO);

        // ... swap_buffers — без изменений ...
    }
}
```

### 4. `run.rs` — без изменений

```rust
while state.tick(...) {}  // ← остаётся, но tick() теперь блокирует
```

## Почему `try_recv()` в `UiNotifier` — не проблема

Частый вопрос: «А `try_recv()` в `UiNotifier::check()` — это же polling?»

Нет. **Polling — это когда `check()` вызывается постоянно.** Если `tick()` вызывается только по событию, то `try_recv()` внутри `check()` — это просто проверка «есть ли уведомления», а не опрос. Он вызывается **один раз на событие**, а не 60 раз в секунду.

Менять `UiNotifier` на `watch::Receiver::has_changed()` можно, но это **не обязательно** для event-driven. Это оптимизация второго порядка.

## Что даёт push на Android

| Метрика | Polling (сейчас) | Push (после) |
|---|---|---|
| CPU в простое | ~5–15% (цикл крутится) | ~0% (поток спит) |
| Батарея | Тратится на пустые кадры | Экономится |
| Рендер | Каждый проход цикла | Только по событию |
| Анимации | 60 FPS всегда | `repaint_delay` от egui |
| Соответствие MVI | Нарушение (polling) | Push-модель |

## Один вопрос, который нужно проверить

**Будит ли `run_on_java_main_thread()` заблокированный `poll_events()`?**

Для **GameActivity** — да. `poll_events()` слушает Java main looper, а `run_on_java_main_thread()` постит callback именно туда.

Для **NativeActivity** — нужно проверить. `poll_events()` использует `ALooper_pollAll()`, а `run_on_java_main_thread()` постит в Java Handler. Это **разные механизмы**. Может потребоваться `ALooper_wake()` или аналог.

Проверка: запустить пример, нажать кнопку, убедиться что UI обновляется. Если нет — добавить `ALooper_wake()` в `Waker`.

## Итого

Push-модель — **правильный выбор** для event-driven UI на Android. Инфраструктура (`Waker`, `StateStore::watch`, `request_repaint`) уже есть. Нужно только **перестать крутить цикл вхолостую**, изменив timeout в `poll_events` и добавив `repaint_delay`. Это 4 файла, ~20 строк, без изменения API. Давай Подробный план выполнения поэтапно от наиболе сложных задача к наименее сложным.
