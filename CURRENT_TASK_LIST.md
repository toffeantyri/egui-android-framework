Это — актуальная дорожная карта для выхода фреймворка на продакшн‑уровень.

---

## Статус выполнения

| Задача | Статус | Дата |
|--------|--------|------|
| 🟥 1. Глобальные статики — удалены | ✅ **Выполнено** | До 2026-07-19 |
| 🟥 2. save/restore навигации | ❌ Не выполнено | — |
| 🟥 3. type-erasure через dyn Any | ❌ Не выполнено | — |
| 🟧 4. Монолитный backend | 🔄 **В работе** | Активная задача |
| 🟧 5. Перегруженный Modifier API | ❌ Не выполнено | — |
| 🟧 6. ComponentContext слишком сложный | ❌ Не выполнено | — |
| 🟨 7. PlatformConfig/AppConfig дублирование | ❌ Не выполнено | — |
| 🟨 8. Хрупкие layout-тесты | ❌ Не выполнено | — |
| 🟨 9. Гибридное хранение PlatformState | ❌ Не выполнено | — |
| 🟩 10. DataLayerHandle заглушка | ❌ Не выполнено | — |
| 🟩 11. Патч egui | ❌ Не выполнено | — |

---

🟥 Приоритет 1 — критические архитектурные проблемы

🟥 1. egui-android-platform-android — глобальное состояние

**Статус: ✅ ВЫПОЛНЕНО**

Что сделано:
- Глобальные `Atomic*` и `OnceLock` (`INSETLEFT`, `SYSTEMTHEME`, `GLOBALVM`, `SYSTEMBARS_COLORS`) — удалены.
- Всё состояние перенесено в `PlatformState` (через `Arc<Mutex<>>`), хранится в backend instance.
- `PlatformState` синхронизируется с `egui::Context::data()` через `store_in_ctx()` / `from_ctx()`.

Что ещё остаётся (🟨 задача 9):
- Два источника истины: `PlatformState` в backend + `egui::Context::data()`. Нужно хранить только в backend.
- JNI указатели (`vm_ptr`, `activity_ptr`) — сырые указатели в публичном API `PlatformState`.

---

🟥 2. egui-android-navigation

Проблема: нет полноценного save/restore
Исправление Waker не влияет на навигацию — она остаётся слабым звеном.

Что не так:
- ChildStack::restore() зависит от порядка элементов.
- Нет сериализации конфигураций.
- Нет типобезопасного состояния компонентов.
- Нет Router/Configuration слоя.

Что делать:
- Ввести ComponentState trait.
- ChildStack::save() → Vec<(C, ComponentState)>.
- ChildStack::restore() → восстановление по конфигурациям.
- Добавить Router + Configuration (как в Decompose).

Приоритет: 🟥🟥🟥
Крейт: egui-android-navigation

---

🟥 3. egui-android-core

Проблема: type‑erasure через dyn Any
Исправление Waker не затрагивает messaging — проблема остаётся.

Что не так:
- Сообщения хранятся как Box<dyn Any + Send>.
- Ошибки типов проявляются только в runtime.
- Невозможно статически проверить корректность сообщений.

Что делать:
- Ввести trait‑bounds: Msg: Clone + Debug + Send + 'static.
- Убрать type‑erasure из DynDispatcher.
- Ввести типобезопасный MessageEnvelope.

Приоритет: 🟥🟥
Крейт: egui-android-core

---

🟧 Приоритет 2 — серьёзные проблемы (не критические, но мешают развитию)

🟧 4. egui-android-platform-android — монолитный backend

**Статус: 🔄 В РАБОТЕ — задача №1 по выполнению**

Проблема: backend монолитный (EGL/IME/Input/SystemBars/run.rs смешаны)

Что не так:
- Backend слишком большой, `backend/mod.rs` смешивает трейт и типы событий.
- run.rs — 440 строк, не тестируемый.
- GlBackend и NativeBackend дублируют код drain'а событий.
- 4 разных ответственности в одном файловом пространстве.

Что делать:
- Разделить `crates/platform-android/src/` на изолированные подмодули без изменения публичного API и логики.

---

### 🗺️ План рефакторинга: Монолитный backend → модульная архитектура

#### Общая стратегия

Разделить backend на подмодули **без изменения публичного API** и **без изменения логики**. Каждый подмодуль получает свою ответственность. `run.rs` остаётся оркестратором, но делегирует специализированным модулям.

Ключевой принцип: **ни один вызов не меняется** — только переезд кода в новые файлы.

#### Текущая карта файлов

```
crates/platform-android/src/
├── lib.rs                 — re-exports + документация по сборке
├── run.rs                 — ГЛАВНЫЙ ЦИКЛ (440 строк): lifecycle, input, render, swap
├── backend/
│   ├── mod.rs             — AndroidBackend trait + BackendEvent + все типы событий
│   ├── gl_backend.rs      — GlBackend (EGL, IME, input drain, waker, platform_state)
│   └── native_backend.rs  — NativeBackend (аналогично, fallback)
├── egl_backend.rs         — EGL FFI + EglState (create/destroy/recreate_surface/swap)
├── input.rs               — InputState + process_input_events() (NativeActivity)
├── insets.rs              — JNI insets + get_pp()
├── platform_state.rs      — PlatformState (Arc<Mutex>)
├── system_bars.rs         — JNI system bars + apply_system_bars_for_theme()
├── theme.rs               — Функции установки clear_color через PlatformState
└── waker.rs               — Реэкспорт Waker
```

#### Что делает run.rs (построчная инвентаризация)

| Строки | Функция | Категория |
|--------|---------|-----------|
| 40-42 | `run()` — точка входа | Lifecycle |
| 45-104 | `run_with_backend()` — инициализация | Init |
| 106-113 | `loop {` — начало главного цикла | Loop |
| 108-281 | `poll_events()` → обработка Lifecycle/Input/BackendEvent | Events |
| 113-170 | InitWindow — EGL init/recreate, restore_state, insets, DPI, PlatformState | Graphics + Lifecycle |
| 172-189 | Resume/Pause/Stop/Destroy — lifecycle | Lifecycle |
| 191-266 | Input/Touch/Key — конвертация BackendEvent → egui::Event | Input |
| 268-280 | TextInput/InsetsChanged/DpiChanged | IME + System |
| 284-293 | destroy_requested — выход из цикла | Lifecycle |
| 295-332 | Painter::new() + RuntimeContext init | Graphics |
| 334-346 | back_pressed — IME check + on_back_pressed | Input + Application |
| 348-351 | rt_ctx.check() — UiNotifier | Runtime |
| 353-436 | Рендеринг: insets, raw_input, frame, tessellate, paint, swap | Graphics |

#### Целевая структура после рефакторинга

```
crates/platform-android/src/
├── lib.rs                 — re-exports (без изменений)
├── run.rs                 — ~50 строк: только run() + run_with_backend() как оркестратор
│
├── event.rs               — НОВЫЙ: BackendEvent, LifecycleEvent, InputEvent, BackendError,
│                            Insets, SurfaceHandle, SystemBarsStyle, TouchPhase, KeyAction
│                            (вынос из backend/mod.rs)
│
├── loop.rs                — НОВЫЙ: RunState — состояние и итерация главного цикла
│
├── graphics.rs            — НОВЫЙ: GraphicsPipeline — создание Painter, рендеринг, scissor
│
├── input_processing.rs    — НОВЫЙ: конвертация BackendEvent в egui::Event (+ IME text)
│
├── lifecycle.rs           — НОВЫЙ: handle_init_window, handle_resume, handle_pause и т.д.
│
├── backend/
│   ├── mod.rs             — только AndroidBackend trait (типы событий → event.rs)
│   ├── gl_backend.rs      — без изменений
│   └── native_backend.rs  — без изменений
│
├── egl_backend.rs         — без изменений (уже выделен)
├── input.rs               — без изменений (NativeActivity input)
├── insets.rs              — без изменений
├── platform_state.rs      — без изменений
├── system_bars.rs         — без изменений
├── theme.rs               — без изменений
└── waker.rs               — без изменений
```

#### Пошаговая последовательность

```
Шаг 1: event.rs     — вынос типов из backend/mod.rs
  └── cargo check + cargo test
Шаг 2: lifecycle.rs — вынос InitWindow/Resume/Pause/Stop/Destroy из run.rs
  └── cargo check + сборка APK + запуск на устройстве
Шаг 3: graphics.rs  — вынос Painter + render из run.rs
  └── cargo check + сборка APK + визуальная проверка
Шаг 4: input_processing.rs — вынос input обработки из run.rs
  └── cargo check + сборка APK + проверка касаний
Шаг 5: loop.rs      — RunState + главный цикл
  └── cargo check + сборка APK + полный cycle-тест
Шаг 6: run.rs       — чистый оркестратор (~50 строк)
  └── cargo check + сборка APK + финальный тест
```

Каждый шаг — отдельный коммит (после разрешения пользователя).
Если на каком-то шаге что-то ломается — шаг откатывается, проблема изолируется.

---

🟧 5. egui-android-ui

Проблема: перегруженный Modifier API
Исправление Waker не влияет на UI‑слой — он всё ещё слишком тяжёлый.

Что не так:
- Слишком много редких модификаторов.
- Анимации смешаны с layout‑модификаторами.
- Нарушение KISS/YAGNI.

Что делать:
- Вынести редкие модификаторы в ui-extras.
- Анимации вынести в ui-animation.
- Оставить минимальный набор базовых модификаторов.

Приоритет: 🟧
Крейт: egui-android-ui

---

🟧 6. egui-android-core

Проблема: ComponentContext слишком сложный
Исправление Waker не затрагивает этот слой.

Что не так:
- Смешение навигации, data layer, back‑dispatcher.
- Три generic‑параметра → сложно объяснить.

Что делать:
- Разделить на:
  - NavigationContext
  - StateContext
  - BackContext
  - DataContext

Приоритет: 🟧
Крейт: egui-android-core

---

🟨 Приоритет 3 — второстепенные проблемы (косметика)

🟨 7. egui-android-platform

Проблема: PlatformConfig смешивает ответственность
Исправление Waker не влияет.

Что не так:
- target_fps относится к runtime.
- log_tag относится к runtime.

Что делать:
- PlatformConfig → только платформенные параметры.
- RuntimeConfig → FPS, логирование.

Приоритет: 🟨
Крейт: egui-android-platform

---

🟨 8. egui-android-ui

Проблема: хрупкие layout‑тесты
Исправление Waker не влияет.

Что не так:
- Тесты завязаны на внутренние детали measure/layout.

Что делать:
- Тестировать только публичный контракт.

Приоритет: 🟨
Крейт: egui-android-ui

---

🟨 9. egui-android-platform-android

Проблема: гибридное хранение PlatformState
Исправление Waker не влияет.

Что не так:
- Состояние хранится в backend и в egui::Context::data().

Что делать:
- Хранить только в backend.

Приоритет: 🟨
Крейт: egui-android-platform-android

---

🟩 Приоритет 4 — низкий

🟩 10. egui-android-runtime

Проблема: DataLayerHandle заглушка
Исправление Waker не влияет.

Что делать:
- Ввести полноценный data layer или удалить заглушку.

---

🟩 11. egui-android-platform-android

Проблема: патч egui
Исправление Waker не влияет.

Что делать:
- Удалить патч, когда upstream исправит баги.

---

📌 Итоговая таблица по крейтам (обновлённая)

| Крейт | Приоритет | Проблемы | Статус |
|-------|-----------|----------|--------|
| platform-android | 🟥🟥🟥🟧🟨🟩 | глобальные статики ✅, JNI/EGL утечки, монолит backend 🔄, run.rs слишком большой 🔄, двойное хранение состояния, патч egui | 🔄 В работе |
| platform | 🟨 | PlatformConfig смешивает ответственность | ❌ |
| runtime | 🟩 | DataLayerHandle заглушка | ❌ |
| core | 🟥🟥🟧 | dyn Any, сложный ComponentContext | ❌ |
| navigation | 🟥🟥🟥 | слабый save/restore, нет Router | ❌ |
| ui | 🟧🟨 | перегруженный Modifier, хрупкие тесты | ❌ |
| framework | 🟩 | косметика | ❌ |

---

## Порядок выполнения (пересмотренный с учётом инвазивности)

```
Фаза 1 — Сейчас (самое инвазивное):
  🔄 Задача 4: Монолитный backend → модульная архитектура (6 шагов)
    └── Шаг 1: event.rs — вынос типов

Фаза 2 — После Фазы 1 (безопасные изменения в других крейтах):
  ❌ Задача 3: type-erasure в core
  ❌ Задача 7: PlatformConfig/AppConfig дублирование
  ❌ Задача 10: DataLayerHandle

Фаза 3 — После Фазы 2 (навигация):
  ❌ Задача 2: save/restore навигации

Фаза 4 — После всего (рефакторинг без изменения логики):
  ❌ Задача 6: ComponentContext разделение
  ❌ Задача 5: Modifier API
  ❌ Задача 8: Хрупкие тесты
  ❌ Задача 9: Гибридное хранение PlatformState
  ❌ Задача 11: Патч egui
```




-------------------------------------------------
Current task plan

Architecture Validation

Прежде чем строить план — системный анализ проблемы.

### Затрагиваемые слои

| Слой | Крейт | Как затрагивается |
|------|-------|-------------------|
| **Platform (Android impl)** | `egui-android-platform-android` | **Целевой** — рефакторинг backend |
| **Platform (абстракция)** | `egui-android-platform` | Не затрагивается — контракт платформы не меняется |
| **Runtime** | `egui-android-runtime` | Не затрагивается — RuntimeContext/Application остаются теми же |
| **Application** | примеры (counter, showcase) | Не затрагиваются — уровень Application не меняется |

### Проверка архитектурных правил

1. ✔ **Однонаправленный поток данных** — не нарушается. Backend только потребляет события и передаёт их наверх.
2. ✔ **Нет новой ответственности у существующих слоёв** — backend остаётся Platform Layer.
3. ✔ **Platform-android не знает больше, чем должен** — наоборот, после разделения он будет знать меньше.
4. ✔ **Нет циклических зависимостей** — разделение backend на подмодули не меняет граф зависимостей.
5. ✔ **Runtime не становится бизнес-слоем** — не затрагивается.
6. ✔ **Не нарушается контракт egui** — не затрагивается.

### Причина проблемы (системный уровень)

`run.rs` (440 строк) и `backend/mod.rs` смешивают **4 разные ответственности** в одном файловом пространстве:

1. **Graphics (EGL)** — `EglState`, `init_graphics()`, `destroy_graphics()`, `swap_buffers()`
2. **Input (NDK)** — `drain_input_events()`, `process_input_events()`, `InputState`
3. **IME (клавиатура)** — `show_keyboard()`, `hide_keyboard()`, `drain_text_input()`
4. **System (Bars, Insets, Theme)** — `system_bars.rs`, `insets.rs`, `theme.rs`

При этом `backend/mod.rs` является файлом, где определён **и трейт `AndroidBackend`**, **и все типы событий** (`BackendEvent`, `LifecycleEvent`, `InputEvent`, `Insets`, `SurfaceHandle`). Это нарушает **Single Responsibility Principle** и **правило "Одна причина изменения"** из архитектурного контракта.

---

# План: Рефакторинг монолитного backend в platform-android

## Общая стратегия

Разделить `crates/platform-android/src/` на изолированные подмодули **без изменения публичного API** и **без изменения логики**. Каждый подмодуль получает свою ответственность. `run.rs` остаётся оркестратором, но делегирует специализированным модулям.

Ключевой принцип: **ни один вызов не меняется** — только переезд кода в новые файлы.

---

## Фаза 0: Инвентаризация текущей структуры

### Текущая карта файлов и их ответственности:

```
crates/platform-android/src/
├── lib.rs                 — re-exports + документация по сборке
├── run.rs                 — ГЛАВНЫЙ ЦИКЛ (440 строк): lifecycle, input, render, swap
├── backend/
│   ├── mod.rs             — AndroidBackend trait + BackendEvent + все типы событий
│   ├── gl_backend.rs      — GlBackend (EGL, IME, input drain, waker, platform_state)
│   └── native_backend.rs  — NativeBackend (аналогично, fallback)
├── egl_backend.rs         — EGL FFI + EglState (create/destroy/recreate_surface/swap)
├── input.rs               — InputState + process_input_events() (используется из run.rs)
├── insets.rs              — JNI insets + get_pp()
├── platform_state.rs      — PlatformState (Arc<Mutex>)
├── system_bars.rs         — JNI system bars + apply_system_bars_for_theme()
├── theme.rs               — Функции установки clear_color через PlatformState
└── waker.rs               — Реэкспорт Waker
```

### Что конкретно делает run.rs (построчная инвентаризация):

| Строки | Функция | Категория |
|--------|---------|-----------|
| 40-42 | `run()` — точка входа | Lifecycle |
| 45-104 | `run_with_backend()` — инициализация: backend, logger, egui_ctx, waker, rt_ctx | Init |
| 106-113 | `loop {` — начало главного цикла | Loop |
| 108-281 | `poll_events()` → обработка Lifecycle/Input/BackendEvent | Events |
| 113-146 | `InitWindow` — EGL init/recreate, restore_state | Graphics + Lifecycle |
| 147-170 | `InitWindow` — system_insets, DPI, PlatformState в ctx | System + Graphics |
| 172-189 | `Resume/Pause/Stop/Destroy` — lifecycle | Lifecycle |
| 191-266 | `Input/Touch/Key` — конвертация BackendEvent → egui::Event | Input |
| 268-280 | `TextInput/InsetsChanged/DpiChanged` | IME + System |
| 284-293 | `destroy_requested` — выход из цикла | Lifecycle |
| 295-332 | `Painter::new()` + `RuntimeContext` init | Graphics |
| 334-346 | `back_pressed` — проверка IME + on_back_pressed | Input + Application |
| 348-351 | `rt_ctx.check()` — UiNotifier | Runtime |
| 353-436 | Рендеринг: insets, raw_input, frame, tessellate, paint, swap_buffers | Graphics |

---

## Фаза 1: Выделение типов событий из backend/mod.rs

### Проблема:
`backend/mod.rs` смешивает:
- Трейт `AndroidBackend`
- Типы событий (`BackendEvent`, `LifecycleEvent`, `InputEvent`)
- Вспомогательные типы (`Insets`, `SurfaceHandle`, `SystemBarsStyle`, `TouchPhase`, `KeyAction`)

### Решение:

**Новый файл**: `crates/platform-android/src/event.rs`

Перенести из `backend/mod.rs`:
- `BackendEvent`
- `LifecycleEvent`
- `InputEvent`
- `TouchPhase`
- `KeyAction`
- `Insets`
- `SurfaceHandle`
- `SystemBarsStyle`
- `BackendError`

`backend/mod.rs` **импортирует** эти типы и либо re-экспортирует их, либо (лучше) использует напрямую из `crate::event`.

### Миграция:
```rust
// backend/mod.rs — после
use crate::event::{BackendEvent, BackendError, ...};
```

Все файлы, использующие `crate::backend::BackendEvent`, переходят на `crate::event::BackendEvent`.

---

## Фаза 2: Выделение RunLoop из run.rs

### Проблема:
`run.rs` — 440 строк, где главный цикл, lifecycle, input, render — всё вместе. Невозможно протестировать ни один сценарий изоляции.

### Решение:

Создать **`crates/platform-android/src/loop.rs`** — чистый модуль главного цикла:

```rust
// loop.rs — итерация главного цикла
pub struct RunState {
    pub input_state: InputState,
    pub egui_painter: Option<egui_glow::Painter>,
    pub last_frame: Instant,
    pub rt_ctx: Option<RuntimeContext>,
    pub rt_ctx_initialized: bool,
    pub destroy_requested: bool,
}

impl RunState {
    pub fn new(config: &AppConfig) -> Self { ... }
    pub fn process_events(
        &mut self,
        backend: &mut dyn AndroidBackend,
        app_instance: &mut dyn Application,
        egui_ctx: &egui::Context,
    ) { ... }  // бывшие строки 108-281
    pub fn render_frame(
        &mut self,
        backend: &mut dyn AndroidBackend,
        app_instance: &mut dyn Application,
        egui_ctx: &egui::Context,
        waker: &Waker,
    ) -> bool { ... }  // бывшие строки 334-436, возвращает destroy_requested
}
```

Каждый публичный метод `RunState`:
- Принимает `&mut dyn AndroidBackend` (через трейт)
- Принимает `&mut dyn Application` (через трейт)
- Работает с собственным состоянием

Это позволяет:
- Тестировать `process_events()` изоляцию без EGL
- Тестировать `render_frame()` с mock backend
- `run_with_backend()` становится ~50 строк (создать RunState + цикл)

**Важно**: пока не тестируем на хосте (platform-android требует `cfg(target_os = "android")`), но архитектурно код становится модульным.

---

## Фаза 3: Выделение Graphics модуля

### Проблема:
Инициализация `egui_glow::Painter` в run.rs (строки 296-332) — это отдельная ответственность, смешанная с lifecycle.

### Решение:

**Новый файл**: `crates/platform-android/src/graphics.rs`

```rust
pub struct GraphicsPipeline {
    pub painter: egui_glow::Painter,
    gl: Arc<glow::Context>,
}

impl GraphicsPipeline {
    pub fn try_new() -> Result<Self, String> { ... }
    
    pub fn render_frame(
        &mut self,
        full_output: &egui::FullOutput,
        window_size: (u32, u32),
        clear_color: (f32, f32, f32),
        insets_pts: (f32, f32, f32, f32),
    ) { ... }
    
    pub fn destroy(&mut self) { ... }
}
```

Перенести:
- Создание `egui_glow::Painter` (run.rs строки 300-327)
- Рендеринг с scissor test (run.rs строки 400-425)
- `paint_and_update_textures` (run.rs строки 417-425)
- `get_proc_address` (run.rs строки 485-504)

---

## Фаза 4: Выделение Input модуля (уже частично есть)

### Проблема:
`input.rs` есть, но он использует `android_activity::input::InputEvent` (для NativeActivity). В run.rs есть дублирующая обработка `BackendEvent::Input` из GlBackend (строки 191-266).

### Решение:

**Новый файл**: `crates/platform-android/src/input_processing.rs`

```rust
pub fn process_backend_input_events(
    backend_events: &[BackendEvent],
    input_state: &mut InputState,
    pixels_per_point: f32,
) {
    // Перенести логику из run.rs строк 191-282
    // Обработка BackendEvent::Input(...), BackendEvent::TextInput, etc.
}
```

Текущий `input.rs` (NativeActivity input через `android_activity::input::InputEvent`) остаётся как есть — он для Native-бэкенда.

---

## Фаза 5: Выделение Lifecycle модуля

### Проблема:
Обработка lifecycle размазана по run.rs в разных местах (InitWindow — строки 113-170, Resume/Pause/Stop/Destroy — строки 172-189, destroy_requested — строки 284-293).

### Решение:

**Новый файл**: `crates/platform-android/src/lifecycle.rs`

Перенести всю логику lifecycle в отдельные функции:
```rust
pub fn handle_init_window(
    backend: &mut dyn AndroidBackend,
    app_instance: &mut dyn Application,
    egui_ctx: &egui::Context,
    has_egl: bool,
    egui_painter: &mut Option<egui_glow::Painter>,
    rt_ctx_initialized: &mut bool,
    rt_ctx: &mut Option<RuntimeContext>,
    waker: &Waker,
) { ... }

pub fn handle_resume(app_instance: &mut dyn Application) { ... }
pub fn handle_pause(app_instance: &mut dyn Application) { ... }
// ...
```

---

## Фаза 6: Итоговая структура после рефакторинга

```
crates/platform-android/src/
├── lib.rs                 — re-exports
├── run.rs                 — ~50 строк: только run() + run_with_backend() как оркестратор
│
├── event.rs               — НОВЫЙ: BackendEvent, LifecycleEvent, InputEvent, BackendError,
│                            Insets, SurfaceHandle, SystemBarsStyle, TouchPhase, KeyAction
│
├── loop.rs                — НОВЫЙ: RunState — состояние и итерация главного цикла
│
├── graphics.rs            — НОВЫЙ: GraphicsPipeline — создание Painter, рендеринг, scissor
│
├── input_processing.rs    — НОВЫЙ: конвертация BackendEvent в egui::Event (+ IME text)
│
├── lifecycle.rs           — НОВЫЙ: handle_init_window, handle_resume, handle_pause и т.д.
│
├── backend/
│   ├── mod.rs             — только AndroidBackend trait (типы событий перенесены в event.rs)
│   ├── gl_backend.rs      — без изменений
│   └── native_backend.rs  — без изменений
│
├── egl_backend.rs         — без изменений (уже выделен)
├── input.rs               — без изменений (NativeActivity input)
├── insets.rs              — без изменений
├── platform_state.rs      — без изменений
├── system_bars.rs         — без изменений
├── theme.rs               — без изменений
└── waker.rs               — без изменений
```

---

## Порядок выполнения и риски

### Шаг 1: `event.rs` — вынос типов из backend/mod.rs

**Риск**: Низкий. Чистый переезд типов. Все импорты меняются с `crate::backend::BackendEvent` на `crate::event::BackendEvent`.
**Проверка**: `cargo check` на хосте (cfg-блоки изолируют, но синтаксис проверяется).

### Шаг 2: `lifecycle.rs` — вынос lifecycle из run.rs

**Риск**: Средний. InitWindow — критический код (EGL init). Нужно аккуратно перенести, ничего не меняя.
**Проверка**: `cargo check` + сборка APK для showcase и запуск на устройстве/эмуляторе.

### Шаг 3: `graphics.rs` — вынос Painter и рендеринга

**Риск**: Средний. `egui_glow::Painter` и scissor test — без `unsafe` не обойтись, но код уже есть.
**Проверка**: Сборка APK + визуальная проверка скролла (scissor test).

### Шаг 4: `input_processing.rs` — вынос обработки input

**Риск**: Низкий. Чистый переезд кода. BackPressed, Touch, IME — без изменений.
**Проверка**: Сборка APK + проверка касаний.

### Шаг 5: `loop.rs` — RunState + главный цикл

**Риск**: Высокий. Это сердце приложения. Ошибка в порядке вызовов = падение.
**Стратегия снижения риска**: Сначала написать `loop.rs` как новую структуру, потом переписать `run_with_backend()` на её использование. Старый код оставить закомментированным до успешной проверки.

### Шаг 6: `run.rs` — чистый оркестратор

**Риск**: Низкий, если шаги 2-5 прошли.

---

## Итоговая пошаговая последовательность

```
Шаг 1: event.rs (вынос типов)
  └── cargo check
Шаг 2: lifecycle.rs (вынос InitWindow/Resume/Pause/Stop/Destroy)
  └── cargo check + сборка APK + запуск
Шаг 3: graphics.rs (вынос Painter + render)
  └── cargo check + сборка APK + визуальная проверка
Шаг 4: input_processing.rs (вынос input обработки)
  └── cargo check + сборка APK + проверка касаний
Шаг 5: loop.rs (RunState + главный цикл)
  └── cargo check + сборка APK + полный cycle-тест
Шаг 6: run.rs (чистый оркестратор)
  └── cargo check + сборка APK + финальный тест
```

Каждый шаг — **отдельный коммит** (после твоего разрешения). Если на каком-то шаге что-то ломается — шаг откатывается, а проблема изолируется и решается до перехода к следующему.
