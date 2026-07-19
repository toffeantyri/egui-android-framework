Это — актуальная дорожная карта для выхода фреймворка на продакшн‑уровень.

---

## Статус выполнения

| Задача | Статус | Дата |
|--------|--------|------|
| 🟥 1. Глобальные статики — удалены | ✅ **Выполнено** | До 2026-07-19 |
| 🟥 2. save/restore навигации | ✅ **Выполнено** (базовый) | 2026-07-19 |
| 🟥 3. type-erasure через dyn Any | ✅ **Выполнено** | 2026-07-19 |
| 🟧 4. Монолитный backend | ✅ **Выполнено** | 2026-07-19 |
| 🟧 5. Перегруженный Modifier API | ❌ Не выполнено | — |
| 🟧 6. ComponentContext слишком сложный | ❌ Не выполнено | — |
| 🟨 7. PlatformConfig/AppConfig дублирование | ✅ **Выполнено** | 2026-07-19 |
| 🟨 8. Хрупкие layout-тесты | ❌ Не выполнено | — |
| 🟨 9. Гибридное хранение PlatformState | ✅ **Выполнено** | 2026-07-19 |
| 🟩 10. DataLayerHandle заглушка | ✅ **Выполнено** | 2026-07-19 |
| 🟩 11. Патч egui | ❌ Не выполнено | — |

---

🟥 Приоритет 1 — критические архитектурные проблемы

🟥 1. egui-android-platform-android — глобальное состояние

**Статус: ✅ ВЫПОЛНЕНО**

Что сделано:
- Глобальные `Atomic*` и `OnceLock` (`INSETLEFT`, `SYSTEMTHEME`, `GLOBALVM`, `SYSTEMBARS_COLORS`) — удалены.
- Всё состояние перенесено в `PlatformState` (через `Arc<Mutex<>>`), хранится в backend instance.
- `PlatformState` — единый источник истины в backend. Синхронизация с `egui::Context::data()` удалена (см. Задачу 9).

Что ещё остаётся:
- JNI указатели (`vm_ptr`, `activity_ptr`) — сырые указатели в публичном API `PlatformState`.

---

🟥 2. egui-android-navigation — save/restore навигации

**Статус: ✅ ВЫПОЛНЕНО** (базовый уровень)

Что сделано:
- `ComponentState` trait — типобезопасное save/restore с ассоциированным типом `State`
- `ChildStack::restore()` — проверяет соответствие конфигураций (prevents data corruption)
- `ChildStack<C>` теперь требует `C: Debug` для логирования несовпадений
- Базовый `ComponentNode::save_state()`/`restore_state()` — дефолт `None`/пусто (безопасное поведение)
- Blanket-impl `ComponentNode for T: Component` больше не переопределяет save_state/restore_state

Что ещё нужно для полной совместимости с Decompose:
- 🆕 2a. Parcelable-сериализация через JNI (android.os.Bundle)
- 🆕 2b. Router/ComponentFactory слой
- 🆕 2c. Рекурсивное сохранение дочерних стеков
- (См. раздел «Decompose — что ещё нужно» ниже)

---

🟥 3. egui-android-core — type-erasure через dyn Any

**Статус: ✅ ВЫПОЛНЕНО**

Что сделано:
- `Component::Message: Clone + Debug + Send + 'static` — статическая гарантия
- `MessageEnvelope<M>` — типобезопасная обёртка (runtime/src/message_envelope.rs)
- `ComponentNode::handle_dyn()` — логирование при ошибке downcast с указанием ожидаемого типа
- Blanket-impl: `save_state()`/`restore_state()` убраны (наследуются из ComponentNode с дефолтом)

---

🟧 Приоритет 2 — серьёзные проблемы

🟧 4. egui-android-platform-android — монолитный backend

**Статус: ✅ ВЫПОЛНЕНО**

Что сделано (6 шагов):
- **Шаг 1**: `event.rs` — вынос типов событий из `backend/mod.rs`
- **Шаг 2**: `lifecycle.rs` — вынос InitWindow/Resume/Pause/Stop/Destroy
- **Шаг 3**: `graphics.rs` — `GraphicsPipeline` (Painter + рендеринг)
- **Шаг 4**: `input_processing.rs` — конвертация BackendEvent → egui::Event
- **Шаг 5**: `loop.rs` — `RunState` + `tick()` — одна итерация главного цикла
- **Шаг 6**: финальная чистка, документация, версия 0.4.0-alpha.1

Итог:
- `run.rs`: 440 → 126 строк (-71%)
- Модулей: 4 → 14 специализированных
- Каждая ответственность в своём файле

---

🟧 5. egui-android-ui

Проблема: перегруженный Modifier API

Что делать:
- Вынести редкие модификаторы в ui-extras.
- Анимации вынести в ui-animation.
- Оставить минимальный набор базовых модификаторов.

Приоритет: 🟧
Крейт: egui-android-ui

---

🟧 6. egui-android-core

Проблема: ComponentContext слишком сложный

Что делать:
- Разделить на: NavigationContext, StateContext, BackContext, DataContext

Приоритет: 🟧
Крейт: egui-android-core

---

🟨 Приоритет 3 — второстепенные проблемы

🟨 7. egui-android-platform

**Статус: ✅ ВЫПОЛНЕНО**

Проблема: PlatformConfig смешивает ответственность — содержит log_tag и target_fps,
которые относятся к Runtime, а не к платформе.

Что сделано:
- `AppConfig` переименован в `RuntimeConfig` (crates/runtime)
- `PlatformConfig` очищен от `log_tag`/`target_fps`, оставлен только `vsync`
- `Application::config()` / `config_mut()` возвращают `&RuntimeConfig`
- Platform-android читает `log_tag`/`target_fps` через `Application::config()`
- Примеры обновлены: `AppConfig` → `RuntimeConfig`

---

🟨 8. egui-android-ui

Проблема: хрупкие layout‑тесты
Что делать: тестировать только публичный контракт.

---

🟨 9. egui-android-platform-android

**Статус: ✅ ВЫПОЛНЕНО**

Проблема: гибридное хранение PlatformState — два источника истины
(backend + egui::Context::data()).

Что сделано:
- `PlatformState::store_in_ctx()` / `from_ctx()` — удалены. Единый источник — backend.
- `theme.rs` — удалён публичный API (`set_clear_color_from`, `current_clear_color`, `current_theme`, `init_system_theme`, `set_theme_override`)
- `system_bars.rs` — `apply_system_bars_for_theme(ctx)` заменён на `apply_system_bars_for_platform_state(&PlatformState)`
- `lifecycle.rs` — при InitWindow: вместо `store_in_ctx()` теперь напрямую устанавливает `clear_color` и system_bars через backend
- `loop.rs` — после `frame()`: чтение `panel_fill` из egui-стиля (установлен Application через `MaterialTheme::apply()`) → `set_clear_color_from()`
- `lib.rs` — убран `pub mod theme`
- Примеры — убраны вызовы `set_clear_color_from()` и `apply_system_bars_for_theme()`

Архитектура: Application задаёт тему через egui::Context, platform-android
сам применяет clear_color и system_bars.

---

🟩 Приоритет 4 — низкий

🟩 10. egui-android-runtime — DataLayerHandle

**Статус: ✅ ВЫПОЛНЕНО**

Проблема: `DataLayerHandle::send()` — пустышка, не отправляет команды.

Что сделано:
- `DataLayerHandle<DataCmd>` — generic-тип поверх `mpsc::Sender<DataCmd>`
- `send()` реально отправляет команду в data layer
- `sender()` — получение сырого `mpsc::Sender` для передачи в `ComponentContext`
- `From<mpsc::Sender<DataCmd>>` для удобной конвертации
- Убран `Default` (требует Sender)

🟩 11. egui-android-platform-android — патч egui

---

## Decompose — что ещё нужно для полной совместимости

### 🆕 2a. Parcelable-сериализация (navigation + platform-android)

**Проблема**: `ChildStack::save()` сохраняет состояние в памяти (`Box<dyn Any + Send>`).
При process kill (Android убивает процесс) состояние теряется.

**В Decompose**: конфигурации сериализуются в `Parcelable` через `Bundle`,
переживают перезапуск процесса.

**Что нужно сделать**:

1. Добавить трейт `ConfigSerializer`:
```rust
/// Сериализация конфигурации навигации в/из Bundle.
pub trait ConfigSerializer<C> {
    fn save(config: &C) -> Vec<i32>;            // или через JNI
    fn restore(data: &[i32]) -> Option<C>;
}
```

2. `ChildStack::save_parcelable()` → сериализует весь стек в Bundle

3. `ChildStack::restore_parcelable()` → восстанавливает стек из Bundle

4. `Application::on_save_state()` → вызывает `save_parcelable()` и сохраняет Bundle через JNI

5. `Application::on_restore_state()` → восстанавливает Bundle через JNI

**Зависимости**: JNI (уже есть), `android.os.Bundle`

**Крейт**: `egui-android-navigation` + `egui-android-platform-android`

---

### 🆕 2b. Router/ComponentFactory слой (navigation)

**Проблема**: `NavigationHost::create_screen()` содержит `match` по всем Route — нарушение OCP.

**В Decompose**: `Router` — отдельный слой, который по конфигурации создаёт компонент.
Config может быть сериализован и восстановлен.

**Что нужно сделать**:

1. Создать `ComponentFactory<C>` trait:
```rust
/// Фабрика компонентов по маршруту.
pub trait ComponentFactory<C> {
    /// Создать компонент для данного маршрута.
    fn create(&self, config: C) -> Box<dyn ComponentNode>;
}
```

2. `NavigationHost::create_screen()` принимает `&dyn ComponentFactory<Route>`

3. Showcase реализует `ComponentFactory<Route>` — без match по всем Route в NavigationHost

**Крейт**: `egui-android-navigation`

---

### 🆕 2c. Рекурсивное сохранение дочерних стеков (navigation)

**Проблема**: `ChildStack::save()` сохраняет только верхний уровень. Если компонент
сам содержит `ChildStack` (например, `NestedScreen`), его состояние не сохраняется.

**В Decompose**: каждая компонента сама знает, как сохранить своё состояние,
включая дочерние стеки. `save()` вызывается рекурсивно.

**Что нужно сделать**:

1. `ComponentNode::save_state()` по умолчанию вызывает `save_state()` на всех дочерних `ChildStack`

2. Добавить в `ComponentNode` метод `save_recursive()`:
```rust
fn save_recursive(&self) -> Vec<(SomeConfig, Option<Box<dyn Any + Send>>)> {
    // сохранить себя + рекурсивно дочерние стеки
}
```

**Крейт**: `egui-android-core` + `egui-android-navigation`

---

### Сводная таблица по Decompose-совместимости

| Возможность | Decompose | У нас | Статус |
|-------------|-----------|-------|--------|
| ChildStack с lifecycle | ✅ | ✅ | Готово |
| ComponentState trait | ✅ | ✅ | Готово |
| Проверка конфигураций при restore | ✅ | ✅ | Готово |
| Типобезопасный save/restore | ✅ | ✅ | Готово |
| Parcelable-сериализация | ✅ | ❌ | 🆕 Задача 2a |
| Router/ComponentFactory | ✅ | ❌ | 🆕 Задача 2b |
| Рекурсивное сохранение стеков | ✅ | ❌ | 🆕 Задача 2c |
| Авто-восстановление после kill | ✅ | ❌ | Нужны 2a + 2b + 2c |

---

📌 Итоговая таблица по крейтам (обновлённая)

| Крейт | Приоритет | Проблемы | Статус |
|-------|-----------|----------|--------|
| platform-android | 🟥🟧🟨🟩🆕 | глобальные статики ✅, монолит backend ✅, JNI/EGL утечки, ~~двойное хранение~~ ✅, патч egui, **Parcelable (2a)** | ⚠️ 4 выполнены |
| platform | 🟨 | ~~PlatformConfig смешивает ответственность~~ ✅ | ✅ Выполнено |
| runtime | 🟩 | ~~DataLayerHandle заглушка~~ ✅ | ✅ Выполнено |
| core | 🟥🟧 | type-erasure ✅, ComponentContext сложный | ⚠️ 1 выполнена |
| navigation | 🟥🟥🟥🆕🆕🆕 | save/restore ✅, **Router (2b)**, **рекурсивное сохранение (2c)** | ⚠️ 1 выполнена |
| ui | 🟧🟨 | перегруженный Modifier, хрупкие тесты | ❌ |
| framework | 🟩 | косметика | ❌ |

---

## Порядок выполнения (обновлённый)

```
✅ Фаза 1 — Выполнено:
  🟥 Задача 1: Глобальные статики — удалены
  🟧 Задача 4: Монолитный backend → модульная архитектура

✅ Фаза 2 — Выполнено:
  🟨 Задача 7: PlatformConfig/AppConfig дублирование (platform + runtime)
  🟩 Задача 10: DataLayerHandle (runtime)
  🟨 Задача 9: Гибридное хранение PlatformState (platform-android)

❌ Фаза 3 — Decompose-совместимость навигации:
  🔲 🆕 Задача 2a: Parcelable-сериализация (navigation + platform-android)
  🔲 🆕 Задача 2b: Router/ComponentFactory (navigation)
  🔲 🆕 Задача 2c: Рекурсивное сохранение стеков (core + navigation)

❌ Фаза 4 — После всего (рефакторинг без изменения логики):
  🔲 🟧 Задача 6: ComponentContext разделение (core)
  🔲 🟧 Задача 5: Modifier API (ui)
  🔲 🟨 Задача 8: Хрупкие тесты (ui)
  🔲 🟩 Задача 11: Патч egui
```
