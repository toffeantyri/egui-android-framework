# Рефакторинг text_selection: сервисная модель вместо глобального слота

> Статус: **план зафиксирован, готов к реализации**.
> Дата: 2026-08-17.
> Крейт: `egui-android-ui` (`crates/ui/src/text_selection/`, `widgets/text.rs`,
> `widgets/text_edit.rs`, `modifier/mod.rs`).
> Протокол: `task-evaluation-full/short`. Порядок: инвазивное → локальное; RED до реализации.
> Предыдущий док (текущее состояние ДО рефакторинга): [`text-selection-plan.md`](./text-selection-plan.md).
> ⚠️ Это НЕ «обновление» старого дока — это **следующий этап**, отменяющий
> модификаторную архитектуру (`Modifier::selectable` + глобальный `TextSurface`).

---

## Цели (ЗАЧЕМ)

**Конечный пользователь Android-приложения:**
1. Выделенный текст больше **не перекрывается полупрозрачным фоном поверх глифов** —
   фон должен ложиться *под* текст (баг `paint_selection_background`).
2. Плавающий тулбар появляется **над** выделением (а не под ним) и имеет
   **горизонтальную** компоновку кнопок.
3. Drag-ручки («капельки») на границах выделения **видно даже вне clip-области**
   текста (сейчас `ui.is_rect_visible` их прячет).
4. В **редактируемых полях** (`TextEdit`) появляется выделение: long-press → слово,
   ручки, тулбар с `Cut`/`Paste` — того сейчас нет вовсе.

**Разработчик фреймворка:**
1. Убрать **глобальный слот `TextSurface`** (скрытое состояние в `Context::data`,
   хрупкий порядок «виджет опубликовал → модификатор прочитал последний», конфликты
   при нескольких selectable и внутри скролла).
2. Устранить **дублирование** логики long-press/драг/тулбара между `Text` и `TextEdit`
   через единый `SelectionCore`.
3. Исправить location-баги рендера в одном месте (под глифами, позиция/ориентация
   тулбара, видимость ручек).
4. Избавиться от «плоского» состояния `AndroidSelectionState`, монолита
   `selection_layer.rs` и лишнего публичного API `Modifier::selectable`.
5. Убрать диагностический `log::info!("SEL..." )` из хот-патча рендера.
6. Зафиксировать и устранить **P0-баг глобального слота**: `surface_slot_id()`
   возвращает ОДИН константный `Id` → при двух `selectable` текстах второй
   публикует поверхность ПОСЛЕ первого и `take_last_published_surface` возвращает
   только его; выделение работает только для последнего виджета (см. P0 в таблице
   проблем ниже).

---

## Ожидаемый результат

- Модуль `crates/ui/src/text_selection/` = `android_behavior.rs`, `selection_core.rs`,
  `render.rs`, `toolbar.rs`, `drag_handles.rs`. Удалены: `surface.rs`,
  `selection_layer.rs`, `state.rs`.
- Публичный API: `Text::selectable(bool)` вместо `Modifier::selectable(bool)`;
  `ModifierNode::Selectable` удалён.
- Тулбар: **над** выделением (`selection.top() - 8`), горизонтальный (`ui.horizontal`),
  фолбэк **под** выделение при верхнем крае экрана.
- Фон выделения: **под** глифами (`egui::text_selection::visuals::paint_text_selection`,
  штатный путь патча egui).
- Ручки: через `Area` + `Order::Foreground`, без `is_rect_visible`.
- `SelectionCore` используется и в `Text`, и в `TextEdit`.
- `TextEdit.read_only` тоже выделяется (Copy + SelectAll).
- Один `log::debug!` на начало выделения, ноль `log::info!`.
- Тесты зелёные: юнит (чистая логика) + интеграционные (рендер без паники).
- Пример `examples/showcase/src/screens/text_selection_screen.rs` мигрирован на новый API.

---

## Проблемы, которые решаем (баги)

| # | Баг | Где | Решение |
|---|-----|-----|---------|
| P0 | Один глобальный слот `TextSurfaceSlot` → при 2+ `selectable` работает только последний | `surface.rs::surface_slot_id()` константный `Id`; `take_last_published_surface` берёт только последний | per-widget состояние через `remember` + `SelectionCore` — каждый текст независим |
| P1 | Фон-прямоугольник рисуется ПОВЕРХ глифов | `selection_layer.rs::paint_selection_background` → `painter.rect_filled` | `paint_text_selection` мутирует mesh: фон вставляется до `glyph_index_start` (под глифами) |
| P2 | Тулбар под выделением и вертикальный | `selection_toolbar.rs::toolbar_anchor` = `bottom()+8`, `ui.vertical`. Нативный Android-тулбар — НАД текстом | `toolbar_anchor` = `top()-8`, `ui.horizontal`, параметр `is_editable` |
| P3 | Ручки невидимы за clip | `drag_handles.rs::draw_handle` → `ui.is_rect_visible` = false | рисовать через `Area` + `Order::Foreground` |

---

## Ограничения (что НЕ делаем)

- ❌ Не трогаем `Modifier`: удаляем только `Modifier::selectable`/`ModifierNode::Selectable`;
  остальные модификаторы (`padding`, `background`, `clickable`, …) не меняются.
- ❌ Не добавляем JNI `ActionMode` / луппу / чтение clipboard через JNI.
- ❌ `Paste` остаётся задизейбленным (нет JNI clipboard read) — только у `TextEdit`, disabled.
- ❌ Mouse/double-click выделение на десктопе — не реализуем (touch-only).
- ❌ Выделение внутри `LazyColumn` / `ScrollArea` — не решаем (P0-ограничение, сохраняется).
- ❌ Не отменяем `Text::id()` / поле `id_salt` — оно **существует** в текущей реализации
  (`widgets/text.rs`), стабильный id важен для нескольких selectable.
  В новом доке оно сохраняется прежним.
- ❌ Не переписываем существующие юнит-тесты `android_behavior`/`drag_handles` без необходимости
  (логика затрагивается только добавлением `draw_handles_in_area`).
- ❌ Не меняем `word-boundary`/`select_word_at` в `android_behavior.rs`.

---

## Архитектура после рефакторинга

```
crates/ui/src/text_selection/
├── mod.rs               ← re-exports
├── android_behavior.rs  ← LongPressState, HandleSide, select_word_at (без изменений)
├── selection_core.rs    ← НОВОЕ: единая логика выделения (Text + TextEdit)
├── render.rs            ← НОВОЕ: paint_galley_with_selection (фон ПОД глифами)
├── toolbar.rs           ← переименован из selection_toolbar.rs + фиксы (над, горизонтальный)
└── drag_handles.rs      ← правка: без is_rect_visible, через Area
```

Удаляются: `surface.rs`, `selection_layer.rs`, `state.rs`, а также плановые
`readonly.rs`/`editable.rs`/`service.rs` (не создаются — заменены `SelectionCore`).
Из `modifier/mod.rs` удаляется `ModifierNode::Selectable` и `Modifier::selectable`.

**Зависимости:** только `egui` + `egui-android-core` (`UiWrapper`) + `egui-android-runtime`
(`Dispatcher`). Нет `platform-android`, нет JNI. Модуль хост-совместим (без `cfg(android)`).

**Архитектурное соответствие:**
- выделение живёт как **локальное UI-состояние** через `remember()` (Уровень 2 гибридной
  модели) — НЕ через MVI/State; корректно.
- нет новых рёбер в DAG; UI слой не узнаёт про Android/сеть/StateStore.
- правило «без `ctx.data_mut()` внутри `render()`» НЕ нарушается (все разделяемые структуры —
  через `remember`/`Arc<RwLock>`).

---

## Ключевые решения

### 1. `SelectionCore` вместо trait-сервисов

Единый конкретный тип, хранимый в `remember`, с общей логикой. Различия (`Cut`/`Paste`)
решаются на уровне виджета (какие кнопки показывать + что возвращать в `BufferCommand`).

```rust
#[derive(Clone, Debug, Default)]
pub struct SelectionCore {
    pub active: bool,
    pub selection: Option<CCursorRange>,
    pub selected_text: String,
    pub selection_rect: Option<egui::Rect>,
}

impl SelectionCore {
    pub fn select_word_at(&mut self, pos, galley, galley_pos, text);
    pub fn drag_handle(&mut self, side, pos, galley, galley_pos);
    pub fn select_all(&mut self, galley, galley_pos, text);
    pub fn handle_toolbar_action(&mut self, action: ToolbarAction) -> BufferCommand;
    pub fn reset(&mut self);
}
```

> Прим. по валидации (З3): `handle_toolbar_action` явно живёт в `SelectionCore` и
> возвращает `BufferCommand`; виджет решает, какое действие отдавать (у `Text` не
> бывает `Cut`/`Paste`).

### 2. Фикс Б1 — self-deadlock при Cut (отложенное действие)

`TextEdit::render` держит `buffer_arc.write()` guard весь кадр (для `te.show`).
Обновлять буфер при Cut прямо там = повторный `write()` на том же `RwLock` =
**deadlock** (`std::sync::RwLock` не реентерабелен).

Решение — `BufferCommand`: сервис НЕ трогает буфер, возвращает команду, а виджет
выполняет мутацию **после** `drop(text_guard)`.

```rust
#[derive(Clone, Debug, PartialEq)]
pub enum BufferCommand {
    DeleteRange { start_char: usize, end_char: usize },
    None,
}
```

Порядок в `TextEdit::render`:
```
te.show(&mut *ui) → TextEditOutput
... selection-логика (влезает в селект/эдит) ...
    let buffer_cmd = sel.handle_toolbar_action(action /* тулбар */);
drop(text_guard);                       // ← guard снят ДО мутации
if let BufferCommand::DeleteRange { start_char, end_char } = buffer_cmd {
    let mut guard = buffer_arc.write()...;  // single write, guard свободен
    // conver char idx → byte; replace_range;
    drop(guard);
    // notify on_changed / on_change_msg ; request_repaint
}
```

**Гарантия Б1:** ни один путь не вызывает `write()` на буфер, пока другой
`write()`-guard жив.

### 3. Фикс Б2 — единая точка вывода текста в `Text`

Раньше текст рисовался безусловно, а selectable-ветка перерисовывала клон галели
поверх. Теперь вывод — одним из двух взаимоисключающих путей:

```
if self.selectable {
    // обработка long-press/drag/toolbar, reads sel.get()
    if let Some(range) = sel.get().selection.filter(|r| !r.is_empty()) {
        paint_galley_with_selection(ui, &galley, text_pos, &range, text_color);  // фон ПОД глифами
        // + ручки, + тулбар
    } else {
        ui.painter_at(rect).galley(text_pos, galley.clone(), text_color);        // обычный
    }
    sel.set(core);   // ← commit состояний в remember (см. З1)
} else {
    ui.painter_at(rect).galley(text_pos, galley.clone(), text_color);
}
```

**Инвариант Б2:** на виджет `Text` за кадр — **ровно один** вызов
`paint_galley_with_selection` или `painter.galley`.

### 4. Цвет при выделении

- Фон выделения: `visuals.selection.bg_fill` (системный).
- Цвет текста в выделении: `visuals.selection.stroke.color` (системный, из патча.
  `paint_text_selection`).
- Цвет вне выделения: `Text::text_color()` либо `visuals.text_color()`.
  Согласуется с поведением egui `Label::selectable`.

---

## Инварианты — чек-лист для code review

- [ ] **Б1:** ни один `buffer_arc.write()` не вызывается, пока живой другой `write()`-guard.
- [ ] **Б1b:** блоки `if response.changed()` и отложенный `DeleteRange` взаимоисключающие
      (не двойной `on_changed` за кадр; см. З5).
- [ ] **Б2:** ровно один `paint_galley_with_selection`/`painter.galley` на `Text` за кадр.
- [ ] **З1:** `sel.set(core)` выполняется в конце selectable-ветки (иначе выделение «не живёт»).
- [ ] **З2:** после отложенного `DeleteRange` вызывается `request_repaint`, иначе локальный
      `on_changed` не перерисует поле.
- [ ] Нет `ctx.data_mut()` внутри `render()` виджетов.
- [ ] Нет `ModifierNode::Selectable` в `modifier/mod.rs`.
- [ ] Нет `surface.rs`, `selection_layer.rs`, `state.rs` в модуле.
- [ ] Тулбар: `toolbar_anchor = selection.top() - 8`, горизонтальный `ui.horizontal`,
      параметр `is_editable`.
- [ ] Порядок `Area`: `draw_handles_in_area` **до** `show_toolbar`; тулбар перекрывает
      ручки при визуальном пересечении (R2).
- [ ] Ручки: без `is_rect_visible`, через `Area` + `Order::Foreground`.
- [ ] **R6:** `paint_galley_with_selection` клонирует `Arc<Galley>` перед
      `paint_text_selection`; оригинал не мутируется (`Arc::make_mut` на клоне).
- [ ] `LongPressState` и `SelectionCore` хранятся в `remember` **в виджете**
      (`Text`/`TextEdit`), рядом друг с другом (R4).
- [ ] `SelectionCore` используется и в `Text`, и в `TextEdit`.
- [ ] `TextEdit.read_only` имеет выделение (Copy + SelectAll) через тот же `SelectionCore`;
      распознавание — по глобальному pointer (`ui.input`), НЕ через `response`;
      `is_editable=false` только в тулбаре (R3).
- [ ] Один `log::debug!` на начало выделения, ноль `log::info!`.
- [ ] Пример мигрирован на `Text::selectable(...)` (ТОЛЬКО на этапе 10, не на этапе 1).

---

## Выявленные на валидации замечания (учесть в реализации)

- **З1 (commit состояния):** в прототипах рендера не хватает `sel.set(core)` в конце
  кадра — без него мутации long-press/drag не сохранятся между кадрами.
- **З2 (repaint после Cut):** голый `on_changed` (без `dispatch`) не вызывает ререндер;
  поле не перерисуется до следующего события. Добавить `request_repaint`.
- **З3 (владелец тулбара):** `SelectionCore::handle_toolbar_action(action) -> BufferCommand`
  — метод живёт в `SelectionCore`, виджет выбирает, какие действия показывать.
- **З4 (пуб-поля):** поля `SelectionCore` должны быть `pub` (читаются из виджета через
  `sel.get().selection`); типаж `Clone + Send + Sync` удовлетворён.
- **З5 (двойной on_changed):** отложенный `DeleteRange` и `response.changed()` в одном
  кадре не пересекаются (Cut сбрасывает буфер, IME-изменений в этот же кадр не будет);
  оформить взаимоисключающе (флаг/guard).
- **З6 (тест горизонтальности):** удалить бесполезный `toolbar_is_horizontal`; вместо него
  тест, что `show_toolbar(..., is_editable=false)` не паникует и не рисует Cut.

---

## Дополнительные замечания финальной проверки (R1–R6)

- **R1 (разрыв этапов 1→6):** на этапе 1 `Text::selectable(bool)` добавляется как поле-заглушка
  **без логики** (рендер как обычный текст). Реальная логика выделения появляется на этапе 6.
  Иначе между этапами selectable-функциональность сломана (модификатор удалён, виджет ещё нет).
- **R2 (порядок Area):** ручки рисуются **до** тулбара; тулбар — поверх ручек (как в нативном Android).
- **R3 (read_only TextEdit / `interactive(false)`):** при `read_only` `te.interactive(false)`
  даёт `Sense::hover()` и поле не берёт фокус. Но **распознавание выделения идёт по глобальному
  pointer** (`ui.input(I.pointer.latest_pos())`) и драгу ручек через `Area`, а НЕ через
  `response`/`interact_pointer_pos` (в текущем коде его вообще нет). Утверждение «нужен отдельный
  `ui.interact`» — **не требуется**: read-only обрабатывается тем же путём `te.show()` (даёт корректный
  `galley`/`galley_pos` даже при non-interactive), тулбар `is_editable=false`, IME-блок исключён
  веткой `if !self.read_only`. Отдельный `ui.interact` ВНЕ добавляем — это лишняя сложность.
- **R4 (где `LongPressState`):** в `remember` в видежете, рядом с `SelectionCore`
  (`("sel_lp", id)` и `("sel_core", id)`).
- **R5 (дубль миграции примера):** этап 1 мигрирует **только тесты**; пример мигрируется на
  этапе 10 (после полной интеграции, когда проверяемы сочетания с `padding`/`background`).
- **R6 (клонирование `Arc<Galley>`):** `paint_galley_with_selection` делает `Arc::clone(galley)` и
  мутирует клон через `paint_text_selection`; оригинал не затрагивается (`Arc::make_mut` вызовет
  копию только на расшарённом Arc).

---

## Неблокирующие нюансы (документировать при реализации, N1–N5)

- **N1 (двойной рендер galley в `TextEdit`):** `te.show(&mut *ui)` уже рисует galley внутри; затем
  `paint_galley_with_selection` перерисовывает её с выделением. В egui последний рендер перекрывает
  предыдущий — визуально корректно, но двойная работа. При необходимости оптимизировать позже
  (`te.measure()` + ручной рендер). P2, НЕ блокирует этап 7.
- **N2 (`Response` из `TextEditOutput`):** `output.response` — это `AtomLayoutResponse`, а не `Response`.
  Доступ к Response: `let response = &output.response.response` (двойная вложенность). Подтверждено в
  патче (`AtomLayoutResponse { pub response: Response, .. }`). Добавить комментарий в код на этапе 7.
- **N3 (`remember`-ключ для `TextEdit`):** `("sel_core", field_id)` / `("sel_lp", field_id)`,
  где `field_id` = `self.field_id.unwrap_or_else(ui.next_auto_id())`. Стабилен при стабильном порядке
  виджетов; в динамических контейнерах (`LazyColumn`) — рекомендовать явный `.id()` (документировать).
- **N4 (`BufferCommand::None` vs `Option`):** оставляем `BufferCommand::None` как вариант enum — явнее,
  чем `Option<BufferCommand::DeleteRange>`.
- **N5 (два `remember` против одного):** `LongPressState` и `SelectionCore` держим в двух отдельных
  `remember` — проще и не трогает `LongPressState` (он по плану не меняется).

---

## План (TDD): инвазивное → локальное

| # | Этап | Что | Инвазивность | Тесты |
|---|------|-----|--------------|-------|
| 1 | 🔴 API-миграция | Удалить `ModifierNode::Selectable` + `Modifier::selectable`. Добавить `Text::selectable(bool)` как **поле-заглушку без логики** (рендер пока обычный). Мигрировать **только тесты** (5 шт). Пример НЕ трогаем (этап 10) | 🔴 публичный API | обновить интеграционные (`Text::selectable(true)` рендерит без паники) |
| 2 | 🔴 RED | `selection_core.rs`: `SelectionCore::select_word_at`, `drag_handle`, `select_all`, `reset`, `handle_toolbar_action->BufferCommand` | 🟢 новый файл | юнит (без egui Context) |
| 3 | 🔴 RED | `render.rs`: `paint_galley_with_selection` (фон ПОД глифами; копирует `Arc<Galley>`) | 🟡 | юнит: корректность через `paint_text_selection` + отсутствие паники |
| 4 | 🔴 RED | `toolbar.rs`: переименовать, `anchor=top()-8`, `ui.horizontal`, `is_editable`, фолбэк под/над | 🟡 | юнит: `toolbar_anchor` над; рендер без Cut для read-only |
| 5 | 🔴 RED | `drag_handles.rs`: убрать `is_rect_visible`, добавить `draw_handles_in_area` (Area, Foreground) | 🟡 | юнит координат ручек (не рисование) |
| 6 | 🟢 GREEN | Интеграция `Text` (фикс Б2): единая точка вывода + `remember`(`LongPressState` рядом с `SelectionCore`) + `sel.set(core)` в конце | 🔴 рендер | интеграционные (рендер без паники; selectable/не-selectable; несколько) |
| 7 | 🟢 GREEN | Интеграция `TextEdit` (фикс Б1): `te.show(&mut *ui)`, `BufferCommand` после `drop`, `request_repaint`. Для `read_only` — тот же путь через `te.show()`+глобальный pointer, НЕ отдельный `ui.interact` (см. R3) | 🔴 рендер | интеграционные + З5 guard |
| 8 | 🔴 Удаление | `surface.rs`, `selection_layer.rs`, `state.rs`; обновить `mod.rs`; убрать `publish_text_surface`/`TextSurface` | 🔴 удаление | обновить re-exports |
| 9 | 🟢 Чистка | Убрать `log::info!("SEL..." )`, оставить один `log::debug!` | 🟢 | — |
| 10 | 🟡 Пример | `text_selection_screen.rs` → `Text::selectable(...)` + миграция сочетаний с padding/background | 🟡 | сборка примера |
| 11 | 🟢 Тесты | новые юнит (`SelectionCore::*`, `BufferCommand::DeleteRange`) + интеграционные (`show_toolbar is_editable=false`, миграция) | 🟢 | см. ТД-секцию |

---

## Покрытие тестами

**Юнит (без egui `Context`, чистая логика):**
- `SelectionCore::select_word_at` → выделяет слово; пустой текст не паникует.
- `SelectionCore::drag_handle` → `HandleSide::Start/End` двигает `primary/secondary`; `selected_text` обновляется.
- `SelectionCore::select_all` → весь текст, `selection_rect` на всю галель.
- `SelectionCore::reset` → сброс всех полей.
- `BufferCommand::DeleteRange` (извлечение байтового диапазона из char-индексов; краевые случаи начала/конца).
- `toolbar_anchor` выше выделения.
- `drag_handles::handle_positions` / `selection_bbox` — уже есть.

**Интеграционные (`crates/ui/tests/text_selection_tests.rs`, рендер без паники):**
- `selectable_text_renders_without_panic` — `Text::selectable(true)`.
- `selectable_text_centered_renders` — `align(Center)` + selectable.
- `non_selectable_text_renders_unchanged` — `selectable(false)`.
- `selectable_text_stable_across_frames` — 3 кадра, состояние стабильно.
- `multiple_selectable_texts_do_not_conflict` — два `Text::selectable(true)` без явного id.
- `textedit_selectable_renders_without_panic` — `TextEdit` + выделение (в меру видимости без event-loop).
- `show_toolbar_readonly_hides_cut` — `show_toolbar(is_editable=false)` не паникует.

> Симуляция реального long-press в egui-тестах ненадёжна (пустые шрифты, pointer-hold);
> логика распознавания закрыта юнит-тестами `android_behavior`.

---

## Оценка

| # | Этап | Время |
|---|------|-------|
| 1 | API-миграция (только тесты) | 0.4 д |
| 2 | `SelectionCore` + unit | 0.5 д |
| 3 | `render.rs` | 0.25 д |
| 4 | `toolbar.rs` | 0.25 д |
| 5 | `drag_handles.rs` | 0.25 д |
| 6 | Интеграция `Text` (Б2) | 0.5 д |
| 7 | Интеграция `TextEdit` (Б1) | 0.75 д |
| 8 | Удаление + `mod.rs` | 0.25 д |
| 9 | Логи | 0.1 д |
| 10 | Пример | 0.25 д |
| 11 | Тесты | 0.5 д |
| **Итого** | | **~4 д** |

> Замечания З1–З5 инкрементальны к коду этапов 2/7 (не отдельные этапы). Чистый **нет**
> `readonly.rs`/`editable.rs`/`service.rs` — один `SelectionCore` вместо трёх файлов
> исходного плана → сокращение с ~4.5 до ~4 д.

---

## Проверочные команды

```bash
# хост-юнит + интеграционные
cargo test -p egui-android-ui

# весь workspace без регрессий
cargo check --workspace
cargo test --workspace

# пример (сборка хоста)
cargo check -p egui-showcase
```

---

## Прогресс реализации

| # | Этап | Статус |
|---|------|--------|
| 1 | API-миграция | ✅ |
| 2 | SelectionCore | ✅ |
| 3 | render.rs | ✅ |
| 4 | toolbar.rs | ✅ |
| 5 | drag_handles.rs | ✅ |
| 6 | Интеграция Text | ✅ |
| 7 | Интеграция TextEdit | ✅ |
| 8 | Удаление | ✅ |
| 9 | Логи | ✅ |
| 10 | Пример | ✅ |
| 11 | Тесты | ✅ |

### Итог этапа 1 (выполнен)

- Удалены `ModifierNode::Selectable`, `Modifier::selectable`, ветка `apply_recursive`, Debug-ветка.
- Добавлен `Text::selectable(bool)` как **поле-заглушку** (рендер пока обычный; логика — этап 6).
- Мигрированы 5 интеграционных тестов `crates/ui/tests/text_selection_tests.rs` на `Text::selectable(...)`.
- **Отступление от R5:** мигрирован и пример `text_selection_screen.rs` (необходимо: `examples/showcase` —
  member workspace, без миграции `cargo check --workspace` ломался бы после удаления `Modifier::selectable`).
- Временный `#![allow(dead_code)]` в `text_selection/mod.rs` — до перестройки модуля (этапы 2–8).
- Проверено: `cargo check --workspace` — чисто; `cargo test -p egui-android-ui` — 267 passed, 0 failed.

> ⚠️ Пред-существующие warnings (не связаны с этапом 1): unused import/variable в
> `crates/ui/tests/layout_tests.rs` и `crates/ui/tests/widget_tests.rs`. Не трогаем.

### Итог этапа 2 (выполнен)

- Создан `selection_core.rs`: `SelectionCore` + `BufferCommand` + `char_range_to_byte_range`.
- Подключён в `text_selection/mod.rs` (реэкспорты `SelectionCore`, `BufferCommand`).
- **Уточнение контракта `handle_toolbar_action` (отклонение от исходного описания):** метод
  возвращает `BufferCommand`, но НЕ выполняет `Copy`/`SelectAll`/`Cut`-сброс и НЕ мутирует состояние.
  Это чистый «что делать с буфером»: `Cut` → `DeleteRange` (если есть выбор); `Copy`/`Paste`/`SelectAll`
  → `None`. Копирование (`copy_text`), `select_all`, reset после применения команды — на уровне виджета
  (этапы 6–7). Причина: для `Copy` виджету нужен текст до сброса; для `SelectAll` — галель.
- Тесты: 13 юнит (`selection_core`), в т.ч. `char_range_to_byte_range` (ascii/cyrillic/edge).
- Проверено: `cargo test -p egui-android-ui` — 280 passed, 0 failed; `cargo check --workspace` — чисто.

### Итог этапа 3 (выполнен)

- Создан `render.rs`: `paint_galley_with_selection` — рисует galley с фоном ПОД глифами через
  `egui::text_selection::visuals::paint_text_selection` (клонирует `Arc<Galley>`, R6).
  Пустой `range` → ничего не рисует.
- Подключён в `text_selection/mod.rs`.
- Тесты: 2 юнит (`empty_range` не паникует; `non_empty` рендерится без паники).
- Проверено: `cargo test -p egui-android-ui` — 282 passed, 0 failed; `cargo check --workspace` — чисто.

### Итог этапа 4 (выполнен)

- Переименован `selection_toolbar.rs` → `toolbar.rs` (см. `mod.rs`/`selection_layer.rs`/`selection_core.rs`).
- `toolbar_anchor` = `top() - 8` (НАД выделением); фолбэк ПОД при верхнем крае (`content_rect.top()+40`).
- `show_toolbar(ctx, id, rect, is_editable)` — 4-й параметр; layout `ui.horizontal` (не `vertical`).
  Для read-only — Copy/SelectAll, Cut/Paste только при `is_editable`. Paste остаётся disabled.
- Тесты: 4 юнит (anchor над, zero-height, no-click→None, `show_toolbar_readonly_hides_cut`).
- Проверено: `cargo test -p egui-android-ui` — 283 passed, 0 failed; `cargo check --workspace` — чисто.

### Итог этапа 5 (выполнен)

- `drag_handles.rs`: из `draw_handle` убрана проверка `ui.is_rect_visible(hit_rect)` — ручки рисуются всегда
  (fix P3).
- Добавлена `draw_handles_in_area(ctx, id, start_pos, end_pos, color) -> (Option<Response>, Option<Response>)`
  — рисует обе ручки через `Area`+`Order::Foreground` (обходит clip-зону текста).
- Тесты: +1 (`draw_handles_in_area_renders_without_panic`) — рендер без паники; итого 4 в модуле.
- Проверено: `cargo test -p egui-android-ui` — 284 passed, 0 failed; `cargo check --workspace` — чисто.

### Итог этапа 6 (выполнен)

- `widgets/text.rs`: `selectable=true` подключает реальную selection-логику (этап 1 был заглушкой).
- Убран `publish_text_surface` из `Text::render` (глобальный слот больше не публикуется; `surface.rs` целиком dead —
  удаляется на этапе 8).
- **Фикс Б2:** единая точка вывода — либо `paint_galley_with_selection` (при активном выделении),
  либо обычный `painter.galley`; никогда оба.
- `remember` держит `LongPressState` (`sel_lp`) и `SelectionCore` (`sel_core`) рядом (R4);
  `sel.set(core)` в конце кадра (З1).
- Распознавание long-press — по глобальному pointer (`ui.input`), NOT через `response` (R3).
- Порядок Area: ручки (`draw_handles_in_area`) ДО тулбара (`show_toolbar`) — тулбар поверх (R2).
- Тулбар для `Text` — `is_editable=false` (read-only: Copy/SelectAll, без Cut/Paste).
- Один `log::debug!` на начало выделения (этап 9 частично).
- Проверено: `cargo test -p egui-android-ui` — 284 passed, 0 failed; `cargo check --workspace` — чисто.

### Итог этапа 7 (выполнен)

- `widgets/text_edit.rs`: `ui.add(te)` → `te.show(&mut *ui)` (возвращает `TextEditOutput` с galley/galley_pos).
- `response` = `&output.response.response` (N2 — `AtomLayoutResponse`); коммент в коде.
- Android-выделение для И editable, и read-only (`is_editable = !read_only`); для read-only `interactive(false)`
  даёт `Sense::hover`, но `te.show` всё равно раскладывает текст (R3).
- **Фикс Б1:** `BufferCommand` (`Cut` → `DeleteRange`) выполняется ПОСЛЕ `drop(text_guard)` через
  `char_range_to_byte_range` + `replace_range`.
- **З2:** после DeleteRange `request_repaint()` — поле перерисовывается без след. внешнего события.
- **З5:** флаг `applied_cut` — если применили DeleteRange, проигнорируем стандартный `response.changed()`
  (нет двойного `on_changed`).
- `is_editing = response.has_focus()` — не активируем выделение слова при IME-вводе.
- Тулбар: для editable показываем Cut; для read-only — только Copy/SelectAll. Copy → `copy_text+reset`,
  SelectAll → `select_all`, Paste — noop (disabled).
- Интеграционные тесты: +2 (`textedit_selectable_renders_without_panic`,
  `textedit_readonly_renders_without_panic`) → в `text_selection_tests` теперь 7.
- Проверено: `cargo test -p egui-android-ui` — 286 passed, 0 failed; `cargo check --workspace` — чисто.

### Итог этапа 8 (выполнен)

- Удалены `selection_layer.rs`, `state.rs`, `surface.rs` (глобальный `TextSurface` слот, монолит,
  `AndroidSelectionState` больше не нужны).
- `text_selection/mod.rs`: убраны `mod selection_layer/state/surface` и реэкспорты
  (`AndroidSelectionState`, `publish_text_surface`, `TextSurface`); снят временный
  `#![allow(dead_code)]`; docstring обновлён (Text + TextEdit).
- Ни один внешний код не ссылался на удалённые символы (grep перед удалением).
- После снятия `allow` в lib НЕТ dead-code warnings (все функции модуля используются).
- Проверено: `cargo test -p egui-android-ui` — 286 passed, 0 failed; `cargo check --workspace` — чисто.

### Итог этапов 9–11 (выполнены)

- **9 (логи):** все `log::info!("SEL...")`/`"SEL-LAYER..."` исчезли вместе с удалением `selection_layer.rs`.
  Остался один `log::debug!` на начало выделения в `Text` и один в `TextEdit`.
- **10 (пример):** `text_selection_screen.rs` уже на `Text::selectable(...)` + сочетания
  `padding`/`background`/`fill_max_width` (мигрирован на этапе 1 как необходимость для workspace;
  после интеграции этапов 6–8 снова проверен — `cargo check --workspace` зелёный).
- **11 (тесты):** юнит-тесты в `selection_core` (13) + `render` (2) + `toolbar` (4) + `drag_handles`
  (4); интеграционные `text_selection_tests` (7). Итог `cargo test -p egui-android-ui` — 286 passed, 0 failed.

---

## Диагностическое логирование pipeline'а (для device-теста)

Добавлены `log::info!("SEL-PIPE [<Text|TextEdit>:<id>] ...")` в ключевые точки пайплайна
выделения/копирования. Уровень `info!` — виден через `android_logger` (logcat) без смены filter.

| Событие | Лог | где |
|---|---|---|
| long-press распознан | `... long-press recognized at <pos>` | `text.rs`, `text_edit.rs` |
| слово под пальцем | `... word selected <text> range=<…>` | `text.rs`, `text_edit.rs` |
| тап вне выделения | `... tap outside -> reset` | `text.rs`, `text_edit.rs` |
| drag ручки | `... drag <Start\|End> pos=<…> -> selected=<…>` | `text.rs`, `text_edit.rs` |
| Copy | `... Copy -> <text>` (передаётся `ui.copy_text`) | `text.rs`, `text_edit.rs` |
| SelectAll | `... SelectAll -> <text>` | `text.rs`, `text_edit.rs` |
| Cut (editable) | `... Cut -> <text>` + `... DELETE bytes <s>..<e> (chars ..) buf_before=…` + `... Cut applied -> buf_after=…` | `text_edit.rs` |

**Замечание:** логи в хот-патче (drag/рендер) — `info!`. На устройстве удобно фильтровать
`adb logcat | grep SEL-PIPE`. Не добавляли логов в `selection_core`/`render`/`toolbar`, чтобы
не спамить юнит-тесты на хосте (логи ведём в виджетах, где есть id).

---

## Рефакторинг ЗАВЕРШЁН
