Это — актуальная дорожная карта для выхода фреймворка на продакшн‑уровень.

---

## Статус выполнения

| Задача | Статус | Дата |
|--------|--------|------|
| 🟥 1. Глобальные статики — удалены | ✅ **Выполнено** | До 2026-07-19 |
| 🟥 2. save/restore навигации | ❌ Не выполнено | — |
| 🟥 3. type-erasure через dyn Any | ❌ Не выполнено | — |
| 🟧 4. Монолитный backend | ✅ **Выполнено** | 2026-07-19 |
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
- Каждая ответственность в своём файле: run (оркестратор), loop (цикл),
  graphics (рендеринг), lifecycle (жизненный цикл),
  input_processing (ввод), event (типы)

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
| platform-android | 🟥🟥🟧🟨🟩 | глобальные статики ✅, монолит backend ✅, JNI/EGL утечки, двойное хранение состояния, патч egui | ⚠️ Две выполнены |
| platform | 🟨 | PlatformConfig смешивает ответственность | ❌ |
| runtime | 🟥🟩 | dyn Any (core), DataLayerHandle заглушка | ❌ |
| core | 🟥🟥🟧 | dyn Any, сложный ComponentContext | ❌ |
| navigation | 🟥🟥🟥 | слабый save/restore, нет Router | ❌ |
| ui | 🟧🟨 | перегруженный Modifier, хрупкие тесты | ❌ |
| framework | 🟩 | косметика | ❌ |

---

## Порядок выполнения (обновлённый)

```
Фаза 1 — Выполнено ✅:
  🟥 Задача 1: Глобальные статики — удалены
  🟧 Задача 4: Монолитный backend → модульная архитектура (6 шагов)

Фаза 2 — Следующая (безопасные изменения):
  🔄 🟥 Задача 3: type-erasure в core (только core, не затрагивает platform-android)
  🔄 🟨 Задача 7: PlatformConfig/AppConfig дублирование (platform + runtime)
  🔄 🟩 Задача 10: DataLayerHandle (runtime)

Фаза 3 — После Фазы 2 (навигация):
  ❌ 🟥 Задача 2: save/restore навигации (navigation)

Фаза 4 — После всего (рефакторинг без изменения логики):
  ❌ 🟧 Задача 6: ComponentContext разделение (core)
  ❌ 🟧 Задача 5: Modifier API (ui)
  ❌ 🟨 Задача 8: Хрупкие тесты (ui)
  ❌ 🟨 Задача 9: Гибридное хранение PlatformState (platform-android)
  ❌ 🟩 Задача 11: Патч egui
```

---

## Рекомендуемая следующая задача

### 🟥 Задача 3: type-erasure через dyn Any (egui-android-core)

**Почему она**: 
- Осталась единственная критическая (🟥) проблема в core-крейте
- Не затрагивает platform-android — можно безопасно делать
- Showcase и Counter продолжат работать через `DynDispatcher` как wrapper
- Минимальная инвазивность: trait bounds + `MessageEnvelope`, примеры не меняются

**Что делать**:
1. Добавить `Msg: Clone + Debug + Send + 'static` в `Component` trait
2. Ввести типобезопасный `MessageEnvelope<M>`
3. `DynDispatcher::wrap<M>()` будет упаковывать в `MessageEnvelope` вместо `Box<dyn Any>`
4. `ComponentNode::handle_dyn()` — downcast через `MessageEnvelope`

**Риск**: Низкий. Примеры не меняют свои типы сообщений.
