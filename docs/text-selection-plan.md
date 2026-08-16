# План: Android-подобное выделение текста (`crates/ui`)

> Финальный, валидированный по фактам кодовой базы план.
> Протокол: `task-evaluation-tdd`. Порядок: инвазивное → локальное; RED до реализации.

## Зачем

**Пользователь продукта:**

- Выделяет статичный текст длинным нажатием, двигает ручки-капельки, копирует через плавающий тулбар — стандартная Android-UX для статичного контента (подписи, текст, списки).

**Разработчик фреймворка:**

- Первый touch-ориентированный слой выделения в `crates/ui` на чистом egui-рендере.
- Без JNI `ActionMode` и без plugin-системы egui (`LabelSelectionState`).
- Позже переиспользуется для `TextEdit` и других контейнеров.

## Задача

Добавить виджету `Text` выделение по моделям Android:

```
long-press → выбрать слово → drag-ручки → плавающий тулбар (Copy / SelectAll)
```

Собственный per-widget стейт (`remember()`), рендер выделения через публичный
`egui::text_selection::visuals::paint_text_selection`.

## Критерии успеха

- [ ] Long-press (≈400мс, дрейф ≤ 12px) выделяет слово по word-boundary.
- [ ] Две drag-ручки расширяют / сужают диапазон.
- [ ] Тулбар над выделением: `Copy` / `SelectAll`, `Paste` disabled.
- [ ] `Copy` кладёт текст в clipboard.
- [ ] Тап вне выделения / новое long-press / клик по кнопке → сброс / перевыделение.
- [ ] `selectable(false)` (по умолчанию) — поведение не меняется.
- [ ] Хост-юнит-тесты (без Android) на всю чистую логику.

## Границы (НЕ делаем)

- ❌ Нативный `ActionMode` через JNI.
- ❌ Лупа (magnifier) при перетаскивании ручки.
- ❌ Выделение в `TextEdit` (редактируемый текст).
- ❌ Выделение в `LazyColumn` / скролле.
- ❌ Triple-tap → абзац.
- ❌ Анимация появления тулбара (fade/slide).
- ❌ Полноценный `Paste` (JNI clipboard read) — только disabled.
- ❌ Mouse-выделение на десктопе (double-click → word) — в P0 не реализуем.

## Архитектура модуля

```
crates/ui/src/
├── text_selection/
│   ├── mod.rs               ← pub mod, реэкспорт типов
│   ├── android_behavior.rs   ← LongPressState + select_word_at
│   ├── drag_handles.rs       ← draw_handle + handle_positions
│   ├── selection_toolbar.rs  ← show_toolbar (Area, Order::Foreground)
│   └── state.rs              ← AndroidSelectionState (remember)
├── widgets/
│   └── text.rs               ← переделка render() при selectable(true)
```

**Зависимости:** только `egui` + `egui-android-core` (UiWrapper), `egui_android_runtime`
(Dispatcher). **Нет** `platform-android`, **нет** plugin-системы egui, **нет** патча egui
(нулевая инвазивность в чужой код).

---

## Решения по красным флагам (валидированы)

### 1. `LabelSelectionState` — Plugin → НЕ используем

`LabelSelectionState` — плагин уровня `Context`/viewport, создан для desktop
mouse-selection. Мы строим **свой touch-слой** (`crates/ui/src/text_selection/`):

- Распознавание long-press — своё, per-widget через `remember()`.
- Отрисовка выделения — публичный `paint_text_selection(&mut Arc<Galley>, ...)`.
- Ручки и тулбар — свои виджеты поверх.

### 2. `select_word_at` приватная → своя реализация

Логика egui (`text_cursor_state.rs`, ~30 строк) дублируется в `android_behavior.rs`
как `pub(crate)`:

```rust
fn select_word_at(text: &str, ccursor: CCursor) -> CCursorRange
```

Word-boundary через `is_word_char(c) = c.is_alphanumeric() || c == '_'`
и `unicode-segmentation`. Патчи egui не трогаем.

### 3. `paint_text_selection` мутирует `&mut Arc<Galley>` → принимаем

В `Text.render()` галель — `Arc<Galley>`. После мутации отрисовка через
`painter.galley(pos, galley, color)` — она клонирует Arc-**указатель**, а не
содержимое, поэтому фон выделения сохраняется.

### 4. Текущий `Text.selectable(true)` = `ui.label()` → переписываем

Найден факт: `ui.label()` создаёт `Label::new(...)` **без** `.selectable(true)`,
т.е. `LabelSelectionState::label_text_selection` сейчас **не вызывается** — в `Text`
нет ни mouse-, ни touch-выделения. Значит mouse-путь «сохранять» нечего; P0 — только
touch. Ветка `selectable(false)` (по умолчанию) **не меняется**.

### 5. `Sense::click_and_drag` → только при `selectable(true)`

```
selectable(false) → Sense::hover()            ← не меняется (default)
selectable(true)  → Sense::click_and_drag()   ← новое
```

Регрессии нет: default `selectable(false)`, поведение существующих `Text` не меняется.

### 6. Конфликт со скроллом в LazyColumn → ограничение P0

- **P0:** документируем ограничение — выделение НЕ работает внутри скролла.
  Дрейф-порог 12px отсекает вертикальный свайп (скролл продолжается).
- **P2 (будущее):** флаг `selection_enabled_in_scroll` / интеграция с `ScrollArea`.

---

## Архитектура: выделение — модификатор (переработано)

> **Решение (финальное):** старый механизм `Text::selectable(bool)` + `render_selectable`
> **полностью удаляется**. Обратная совместимость не нужна. Новый механизм —
> **`Modifier::selectable(true|false)`**, а `Text` остаётся обычным виджетом с одним
> рендером.

### Модель (масштабируемая + переиспользуемая)

Выделение выносится из виджета в модификатор по схеме «[контент публикует →
модификатор рисует]», аналогичной уже существующему в фреймворке паттерну
`TextEdit → ImeEditorStateSlot`:

1. **`TextSurface`** (`text_selection/surface.rs`) — что текстовый виджет публикует:
   `{ galley: Arc<Galley>, galley_pos: Pos2, text: String }`.
2. **Активный слот** — `TextSurfaceSlot = Arc<RwLock<Option<TextSurface>>>` в
   `Context::data` (паттерн `remember`/`Arc<RwLock>`, без deadlock).
3. **`Text`** — единый рендер: рисует текст как сейчас и в конце вызывает
   `publish_text_surface(ui, TextSurface{..})` (публикует «активную» поверхность).
4. **`Modifier::selectable(true|false)`** (`ModifierNode::Selectable`):
   - `false` → просто `rest(ui, dispatch)`;
   - `true` → как `Clickable`: рендерит контент в `child_ui`, затем читает
     опубликованную поверхность, покрывает область `Sense::click_and_drag()` и
     ведёт всю selection-логику (long-press → слово → ручки → тулбар) поверх.

### Масштабируемость

- Модификатор НЕ знает про `Text` — ему нужен любой «публикатор поверхности».
  Будущие текстовые виджеты (`TextEdit` и т.п.), вызывающие `publish_text_surface`,
  автоматически получают выделение через тот же модификатор в любых контейнерах.
- Выделение рисуется **поверх** уже отрисованного контента (фон-отверг + ручки +
  тулбар), без перерисовки текста. Один «активный» слот на кадр (паттерн
  единственного активного редактора) — расширение до словаря по id возможно позже.

### Что меняется

- `widgets/text.rs`: удалить `selectable` field/метод, `render_selectable`;
  единый `render` + `publish_text_surface`.
- `modifier/mod.rs`: добавить `ModifierNode::Selectable(bool)`,
  `Modifier::selectable(bool)`, ветку в `apply_recursive`.
- `text_selection/surface.rs`: `TextSurface`, слот, `publish/take_last_published`.
- Демо и интеграционные тесты обновить на `.modifier(Modifier::new().selectable(true))`.

---

## Фиксации координат и Id (шаги 0–1)

### galley_pos и `align`

Паттерн `Label::layout_in_ui`:

```rust
let galley = /* layout через LayoutJob, halign = self.align */;
let (rect, response) = ui.allocate_exact_size(galley.size(), sense);

let galley_pos = match self.align.unwrap_or(Align::LEFT) {
    Align::LEFT   => rect.left_top(),
    Align::Center => rect.center_top(),
    Align::RIGHT  => rect.right_top(),
};
```

- `align` влияет **только** на `galley_pos.x` (согласован с `halign` галели).
- Выделение рисуется внутри галели (galley-local координаты) — дополнительный сдвиг не нужен.
- Ручки: `handle_pos = galley_pos + galley.pos_from_cursor(cursor).center()`.
- **Внимание:** отрисовка **НЕ через** `painter_at(rect)` (клип по rect обрежет ручки,
  которые выступают ~24px под строку). Правильный путь:
  `ui.painter().add(TextShape::new(galley_pos, galley, color))`.

### Стабильный Id (`id_salt`) в `Text`

```rust
pub struct Text {
    // ... существующие поля ...
    id_salt: Option<egui::Id>,  // НОВОЕ
}

impl Text {
    pub fn id(mut self, id: egui::Id) -> Self {
        self.id_salt = Some(id);
        self
    }
}
```

В `render` при `selectable(true)`:

```rust
let widget_id = self.id_salt.unwrap_or_else(|| ui.next_auto_id());
let response = ui.interact(rect, widget_id, Sense::click_and_drag());
let sel_state = remember(ui, ("android_sel", widget_id), AndroidSelectionState::default);
```

> Для `selectable(true)` в динамических контейнерах (`LazyColumn`, условный рендер)
> задавайте явный `id`. Без явного id стабильность гарантирована только при неизменном
> порядке виджетов.

---

## Поведение

### ✅ Ожидаемое

- Держим палец 400мс без дрейфа на слове → слово выделено, показаны ручки и тулбар.
- Тянем ручку `End` → расширяем; `Start` → сужаем / переворачиваем.
- `Copy` → текст в clipboard; клик по кнопке → скрываем тулбар.
- Тап вне выделения → сброс, тулбар скрыт.
- Повторное long-press → перевыделение слова.

### ❌ Не ожидаемое / edge cases

- Пустой текст: `select_word_at("")` → `CCursorRange::one` без паники.
- Дрейф > 12px до 400мс → отмена. Вертикальный свайп = скролл, не выделение.
- Уже активное выделение + повторный held — без переназначения внутри drag-режима.
- Выделение нулевой длины (start == end) → тулбар/ручки не показывать.
- **Deadlock:** не держать `RwLock` guard из `remember()` при вызове `ui.*` / `draw_handle` / `show_toolbar`.
- `paint_text_selection` вызывается только при непустом `selection` (иначе мутирует galley лишний кадр).

### 🐛 Из реальных багов / регресс-источник

- Регресс: `selectable(false)` должен остаться прежним (рендер без выделения).
- «Ручки не обрезаются» — проверяем через отсутствие rect-клиппинга в `selectable(true)`.
- `selectable(true)` в динамическом контейнере без явного `id` может сбрасывать состояние — документируем.

---

## План (TDD)

| # | Фаза | Шаг | Инвазивность | Тесты |
|---|------|-----|--------------|-------|
| 0 | 🅰️ АБСТРАКЦИЯ | `text_selection/mod.rs`: типы `LongPressState`, `HandleSide`, `AndroidSelectionState`, `select_word_at(text, CCursor)`; `id_salt: Option<Id>` в `Text` | 🔴 | — |
| 1 | 🔴 RED | `android_behavior.rs`: `LongPressState::update` (400мс, дрейф ≤12px, отмена, single-trigger, reset) | 🟡 чистая логика | 4–5 |
| 2 | 🔴 RED | `android_behavior.rs`: `select_word_at` (слово, пунктуация, границы, пустой, emoji) | 🟡 чистая логика | 4–5 |
| 3 | 🔴 RED | `drag_handles.rs`: `handle_positions` из `sorted_cursors()` + `galley.pos_from_cursor`; `draw_handle` (геометрия, hit-зона) | 🟡 чистая логика | 3 |
| 4 | 🔴 RED | `selection_toolbar.rs`: `show_toolbar` действие + позиция (`__run_test_ui`) | 🟡 | 2–3 |
| 5 | 🟢 GREEN | Реализация шагов 1–4 (чистая логика, без рендера `Text`) | 🔴 | проходят |
| 6 | 🔴 RED | Интеграция `Text.render()`: галель + halign → `paint_text_selection` → `ui.painter().add(TextShape)` **без rect-клипа**; long-press→слово; drag-ручки; тулбар; copy | 🔴 рендер | 3–4 |
| 7 | 🟢 GREEN | Довести `Text` до прохождения; регресс `selectable(false)` + «ручки не обрезаются» | 🔴 | проходят |
| 8 | 🔧 REFACTOR | Константы, anti-duplicate word-boundary, документация scroll-limit | 🟡 | проходят |

**Оценка MVP: ~3 дня.** Полный `Paste` (JNI clipboard) — P1, отдельно.

## Проверочные команды

```bash
# хост-юнит-тесты (чистая логика без Android)
cargo test -p egui-android-ui

# весь workspace (без регрессий)
cargo check --workspace
cargo test --workspace
```

---

## Прогресс реализации (TDD-цикл)

| # | Фаза | Шаг | Статус | Комментарий |
|---|------|-----|--------|-------------|
| 0 | 🅰️ АБСТРАКЦИЯ | типы + зависимость `unicode-segmentation` | ✅ | `text_selection/` создан |
| 1 | RED→GREEN | `LongPressState::update` / `update_raw` | ✅ | 5 юнит-тестов |
| 2 | RED→GREEN | `select_word_at` (word-boundary) | ✅ | 5 юнит-тестов |
| 3 | RED→GREEN | `handle_positions` / `draw_handle` | ✅ | 3 юнит-теста |
| 4 | RED→GREEN | `show_toolbar` / `toolbar_anchor` (вертикально) | ✅ | 3 юнит-теста |
| 5 | 🟢 GREEN | `AndroidSelectionState` (state.rs) | ✅ | определен в абстракции |
| 6 | 🔴→🟢 | РЕФАКТОРИНГ: выделение → `Modifier::selectable(bool)` | ✅ | убран `Text::selectable`, `render_selectable` |
| 7 | 🟢 GREEN | `TextSurface`-слот + `Modifier` selection-слой | ✅ | интеграционные тесты (5) |
| 8 | 🔧 REFACTOR | Очистка/документация | ⏳ | |
| — | 🖥️ Демо | `ShowcaseScreen` в `examples/showcase` | ✅ | `Route::TextSelection`, фабрика, home |

**Проверено:** `cargo test -p egui-android-ui` — 266 тестов зелёные (в т.ч. 5 integration
`text_selection_tests`); `cargo check --workspace` — без ошибок/warnings.

**Замечания по интеграции:**
- `__run_test_ui` ставит пустые шрифты — тесты галели/рендера используют `Context::default()`.
- Симуляция long-press pointer-hold ненадёжна в egui-тестах; распознавание покрыто юнит-тестами.
- `Modifier::selectable` рисует выделение ПОВЕРХ текста (фон-прямоугольники + ручки + тулбар),
  без перерисовки galley; один «активный» слот поверхности на кадр.
- Старый `Text::selectable(bool)` и `Text::id()` полностью удалены (без обратной совместимости).
