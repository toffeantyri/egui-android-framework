# Контракт реализации: виджет `TextEdit` для egui-android-framework

## 1. Обзор

Реализовать виджет `TextEdit` в крейте `ui` — обёртку над `egui::TextEdit` из патчей, интегрированную в MVI-архитектуру фреймворка. Референс поведения — Jetpack Compose `TextField` / `BasicTextField`.

**Одно предложение:** виджет принимает `value: &str` из State, при изменении вызывает пользовательский callback (замыкание или Message), при фокусе управляет клавиатурой через `egui::Context` data.

---

## 2. Архитектурные ограничения

| Правило | Пояснение |
|---|---|
| DAG без циклов | `ui` зависит от `core` и `runtime`, НЕ зависит от `platform-android` |
| Единственная точка изменения | `store.update()` через dispatch |
| Push-модель | Никакого polling. Клавиатура управляется по событию фокуса |
| Запрещено `OnceLock` для Sender | Не использовать |
| Запрещено `std::process::exit(0)` | Не использовать |
| Запрещено `Vec<Message>` из View | View возвращает `()` |
| Комментарии и логи | На русском языке |
| Контейнеры | Compose-like замыкания (не builder) |

---

## 3. Файлы для создания / изменения

### Создать

| Файл | Назначение |
|---|---|
| `crates/ui/src/widgets/text_edit.rs` | Виджет `TextEdit<M>` |

### Изменить

| Файл | Изменение |
|---|---|
| `crates/ui/src/widgets/mod.rs` | Добавить `mod text_edit;` и `pub use text_edit::TextEdit;` |
| `crates/ui/src/lib.rs` | Добавить `TextEdit` в `pub use widgets::{...}` |

### НЕ изменять

- `crates/platform-android/*` — клавиатура уже работает через `BackendEvent::TextInput` → `egui::Event::Text`
- `crates/core/*` — не нужно
- `crates/runtime/*` — не нужно
- `patches/egui/*` — `egui::TextEdit` уже полностью функционален

---

## 4. API виджета

```rust
// crates/ui/src/widgets/text_edit.rs

use egui_android_core::{widget::Widget, UiWrapper};
use egui_android_runtime::Dispatcher;

/// Тип клавиатуры для IME.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum KeyboardType {
    #[default]
    Text,
    Email,
    Phone,
    Number,
    Password,
    Uri,
}

/// Действие кнопки IME (Done / Search / Next / Go).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ImeAction {
    #[default]
    Done,
    Search,
    Next,
    Go,
}

/// Виджет текстового ввода.
///
/// Обёртка над `egui::TextEdit`, интегрированная в MVI-архитектуру.
///
/// # Пример
/// ```ignore
/// TextEdit::new(&state.email)
///     .hint("Email")
///     .single_line()
///     .on_change_msg(|v| Msg::EmailChanged(v))
///     .render(ui, dispatch);
/// ```
pub struct TextEdit<M> {
    /// Текущее значение (читается из State).
    value: String,

    /// Текст-подсказка (placeholder), отображается когда поле пустое.
    hint_text: String,

    /// Однострочный режим (по умолчанию true).
    single_line: bool,

    /// Маска пароля — символы заменяются на ●.
    password: bool,

    /// Максимальное количество строк (только для multiline).
    /// None = без ограничения.
    max_lines: Option<usize>,

    /// Лимит символов. None = без ограничения.
    char_limit: Option<usize>,

    /// Только чтение (без возможности редактирования).
    read_only: bool,

    /// Замыкание, вызываемое при каждом изменении текста.
    /// Пользователь сам решает: dispatch, remember, ничего.
    on_changed: Option<Box<dyn Fn(&str)>>,

    /// Альтернатива on_changed: замыкание, формирующее Message
    /// из нового значения для dispatch в Store.
    on_changed_msg: Option<Box<dyn Fn(String) -> M>>,

    /// Вызывается при IME action (Done/Search) или потере фокуса.
    /// По умолчанию Done → скрыть клавиатуру.
    on_submit: Option<Box<dyn Fn(&str)>>,

    /// Тип клавиатуры.
    keyboard_type: KeyboardType,

    /// Действие кнопки IME.
    ime_action: ImeAction,
}
```

### Builder-методы (все принимают `self`, возвращают `Self`)

```rust
impl<M: 'static> TextEdit<M> {
    /// Создать новый TextEdit с начальным значением.
    pub fn new(value: impl Into<String>) -> Self;

    /// Текст-подсказка.
    pub fn hint(mut self, text: impl Into<String>) -> Self;

    /// Однострочный режим (по умолчанию).
    pub fn single_line(mut self) -> Self;

    /// Многострочный режим.
    pub fn multiline(mut self) -> Self;

    /// Маска пароля.
    pub fn password(mut self) -> Self;

    /// Максимум строк (multiline).
    pub fn max_lines(mut self, n: usize) -> Self;

    /// Лимит символов.
    pub fn char_limit(mut self, n: usize) -> Self;

    /// Только чтение.
    pub fn read_only(mut self) -> Self;

    /// Замыкание при изменении (локальная логика).
    pub fn on_changed<F: Fn(&str) + 'static>(mut self, f: F) -> Self;

    /// Замыкание, формирующее Message при изменении (MVI-поток).
    pub fn on_change_msg<F: Fn(String) -> M + 'static>(mut self, f: F) -> Self;

    /// Замыкание при submit (Done / потеря фокуса).
    pub fn on_submit<F: Fn(&str) + 'static>(mut self, f: F) -> Self;

    /// Тип клавиатуры.
    pub fn keyboard_type(mut self, kt: KeyboardType) -> Self;

    /// Действие кнопки IME.
    pub fn ime_action(mut self, action: ImeAction) -> Self;
}
```

### Реализация Widget

```rust
impl<M: Clone + Send + 'static> Widget<M> for TextEdit<M> {
    fn render(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<M>) {
        // 1. Клонировать value для мутабельного доступа
        // 2. Построить egui::TextEdit (singleline/multiline)
        // 3. Применить настройки (hint, password, char_limit, interactive)
        // 4. Показать через ui.add(...)
        // 5. Обработать response:
        //    a. gained_focus → показать клавиатуру
        //    b. lost_focus → скрыть клавиатуру + on_submit
        //    c. changed → on_changed / on_changed_msg
        // 6. Обработать IME action (Enter для multiline, Done для singleline)
    }
}
```

---

## 5. Поведение по кейсам

### 5.1 Однострочный ввод (singleline)

- Текст длиннее поля → **горизонтальный скролл** (egui `TextEdit::singleline()` делает это автоматически через `clip_text(true)`).
- **Нет переноса строк**. Enter не создаёт новую строку.
- Enter / Done → вызывает `on_submit` + скрывает клавиатуру.

### 5.2 Многострочный ввод (multiline)

- Текст не помещается по ширине → **перенос на следующую строку** (word wrap). egui `TextEdit::multiline()` делает это автоматически.
- Текст не помещается по высоте → **вертикальный скролл**. egui `TextEdit::multiline()` скроллит автоматически.
- Enter → новая строка (стандартное поведение egui).
- `max_lines` → ограничить высоту через `desired_rows()` на egui TextEdit.

### 5.3 Маска пароля

- `password(true)` → `egui::TextEdit::password(true)`.
- Символы отображаются как `●`. Копирование заблокировано (egui делает это автоматически).

### 5.4 Read-only

- `read_only(true)` → `egui::TextEdit::interactive(false)`.
- Текст отображается, но не редактируется. Клавиатура не открывается.

### 5.5 Лимит символов

- `char_limit(n)` → `egui::TextEdit::char_limit(n)`.

---

## 6. Управление клавиатурой

### Архитектура

Аналог `LocalSoftwareKeyboardController` из Jetpack Compose. Платформа регистрирует callback в `egui::Context` data, виджет получает через `ui.ctx().data()`.

### Структура callback (регистрируется платформой)

```rust
/// Регистрируется в platform-android при инициализации.
/// Хранится в egui Context data по Id("egui_keyboard_controller").
struct KeyboardController {
    show: Arc<dyn Fn() + Send + Sync>,
    hide: Arc<dyn Fn() + Send + Sync>,
}
```

### Регистрация (platform-android, при инициализации)

```rust
// В run.rs или в GlBackend::init / NativeBackend::init:
let kb = KeyboardController {
    show: Arc::new(move || app.show_soft_input(false)),
    hide: Arc::new(move || app.hide_soft_input(false)),
};
egui_ctx.data_mut(|d| {
    d.insert_temp(Id::new("egui_keyboard_controller"), kb);
});
```

### Использование в TextEdit

```rust
// При gained_focus:
if response.gained_focus() {
    ui.ctx().data(|d| {
        if let Some(kb) = d.get_temp::<KeyboardController>(Id::new("egui_keyboard_controller")) {
            (kb.show)();
        }
    });
}

// При lost_focus:
if response.lost_focus() {
    ui.ctx().data(|d| {
        if let Some(kb) = d.get_temp::<KeyboardController>(Id::new("egui_keyboard_controller")) {
            (kb.hide)();
        }
    });
}
```

### Важно

- `KeyboardController` определяется в `ui` крейте (структура + Id-константа).
- `platform-android` заполняет callback при инициализации.
- Если callback не зарегистрирован (десктоп, тесты) — **не паниковать**, просто пропустить.
- `read_only` виджет **не открывает** клавиатуру.

---

## 7. Обработка IME action

### Поведение по умолчанию

| ImeAction | Поведение |
|---|---|
| `Done` | Скрыть клавиатуру. Вызвать `on_submit` если задан. |
| `Search` | То же что Done. |
| `Next` | Скрыть клавиатуру. (Передача фокуса — P2, пока просто скрыть.) |
| `Go` | То же что Done. |

### Реализация

Для **singleline**: egui TextEdit по умолчанию обрабатывает Enter как submit. Нужно:
1. Перехватить `Key::Enter` через `response.lost_focus()` или `event_filter`.
2. Вызвать `on_submit`.
3. Скрыть клавиатуру.

Для **multiline**: Enter создаёт новую строку. Submit происходит **только при потере фокуса** (тап вне поля).

### Кастомная логика Done

Пользователь задаёт через `on_submit`:

```rust
TextEdit::new(&state.query)
    .hint("Поиск")
    .ime_action(ImeAction::Search)
    .on_submit(|text| {
        dispatch.dispatch(Msg::Search(text.to_owned()));
    })
    .render(ui, dispatch);
```

Если `on_submit` не задан → Done просто скрывает клавиатуру.

---

## 8. MVI-интеграция

### Паттерн 1: полный MVI (каждый символ → Store)

```rust
// В view:
TextEdit::new(&state.email)
    .hint("Email")
    .on_change_msg(|v| Msg::EmailChanged(v))
    .render(ui, dispatch);

// В handle:
Msg::EmailChanged(new_value) => {
    ctx.store.update(|s| s.email = new_value);
}
```

### Паттерн 2: локальный remember + submit

```rust
let local = remember(ui, "email_input", || String::new());

TextEdit::new(&local.get())
    .hint("Email")
    .on_changed({
        let local = local.clone();
        move |v| local.set(v.to_owned())
    })
    .on_submit(move |v| {
        dispatch.dispatch(Msg::Submit(v.to_owned()));
    })
    .render(ui, dispatch);
```

### Паттерн 3: кастомная логика

```rust
TextEdit::new(&state.phone)
    .hint("Телефон")
    .on_changed(move |v| {
        // Валидация, форматирование и т.д.
        if v.len() <= 18 {
            dispatch.dispatch(Msg::PhoneChanged(v.to_owned()));
        }
    })
    .render(ui, dispatch);
```

### Приоритет callback

Если заданы и `on_changed`, и `on_changed_msg` — вызываются **оба** (сначала `on_changed`, потом `on_changed_msg`). Это позволяет комбинировать локальную логику с MVI.

---

## 9. Скроллинг (как в Compose)

| Режим | Скролл | Реализация |
|---|---|---|
| `singleLine = true` | Горизонтальный, курсор всегда виден | egui `TextEdit::singleline()` — автоматически |
| `multiline`, `maxLines = N` | Вертикальный до N строк, потом скролл | egui `TextEdit::multiline().desired_rows(N)` |
| `multiline`, без `maxLines` | Вертикальный без ограничения | egui `TextEdit::multiline()` — автоматически |

**Не нужно** оборачивать в `ScrollArea` вручную. egui TextEdit multiline скроллит сам и автоскроллит к курсору при вводе.

---

## 10. Маски (P0 и P2)

### P0: пароль

```rust
TextEdit::new(&state.password)
    .hint("Пароль")
    .password(true)
    .render(ui, dispatch);
```

Реализация: `egui::TextEdit::password(true)` — уже работает.

### P2: кастомные маски (телефон, email)

Принципиальная возможность через `layouter`. В контракте P0 **не реализуем**, но API не должен блокировать добавление в будущем.

В будущем:

```rust
// P2 — НЕ в текущей реализации
TextEdit::new(&state.phone)
    .mask(|raw| format_phone(raw))
    .render(ui, dispatch);
```

---

## 11. Тесты

### Обязательные unit-тесты в `crates/ui/tests/widget_tests.rs`

```rust
#[test]
fn test_text_edit_renders_singleline() { /* не паникует */ }

#[test]
fn test_text_edit_renders_multiline() { /* не паникует */ }

#[test]
fn test_text_edit_password() { /* не паникует */ }

#[test]
fn test_text_edit_read_only() { /* не паникует */ }

#[test]
fn test_text_edit_hint() { /* не паникует */ }

#[test]
fn test_text_edit_char_limit() { /* не паникует */ }

#[test]
fn test_text_edit_max_lines() { /* не паникует */ }

#[test]
fn test_text_edit_on_changed_callback() { /* не паникует */ }

#[test]
fn test_text_edit_on_change_msg_callback() { /* не паникует */ }

#[test]
fn test_text_edit_on_submit_callback() { /* не паникует */ }

#[test]
fn test_text_edit_keyboard_type() { /* не паникует */ }

#[test]
fn test_text_edit_ime_action() { /* не паникует */ }

#[test]
fn test_text_edit_is_widget() { /* принимает dyn Widget */ }

#[test]
fn test_text_edit_in_column() { /* рендер внутри Column */ }

#[test]
fn test_text_edit_in_row() { /* рендер внутри Row */ }
```

### Паттерн тестов

Использовать существующий `with_ui()` хелпер из `widget_tests.rs`:

```rust
fn with_ui(f: impl FnOnce(&mut UiWrapper)) {
    let f = RefCell::new(Some(f));
    let ctx = egui::Context::default();
    let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let f = f.borrow_mut().take().unwrap();
            f(&mut UiWrapper::new_unconstrained(ui));
        });
    });
}
```

---

## 12. Примеры использования

### Пример 1: форма логина

```rust
fn login_view(state: &LoginState, ui: &mut UiWrapper, dispatch: &Dispatcher<Msg>) {
    let c = &Theme::current_from_ui(ui).colors;

    Column::new().show(ui, dispatch, |ui, dispatch| {
        TextEdit::new(&state.email)
            .hint("Email")
            .single_line()
            .keyboard_type(KeyboardType::Email)
            .ime_action(ImeAction::Next)
            .on_change_msg(|v| Msg::EmailChanged(v))
            .modifier(Modifier::new().fill_max_width().padding(8.0))
            .render(ui, dispatch);

        TextEdit::new(&state.password)
            .hint("Пароль")
            .single_line()
            .password(true)
            .keyboard_type(KeyboardType::Password)
            .ime_action(ImeAction::Done)
            .on_change_msg(|v| Msg::PasswordChanged(v))
            .on_submit(|_| dispatch.dispatch(Msg::Login))
            .modifier(Modifier::new().fill_max_width().padding(8.0))
            .render(ui, dispatch);

        Button::new("Войти")
            .on_click(Msg::Login)
            .theme_colors(c.primary)
            .text_color(c.on_primary)
            .modifier(Modifier::new().fill_max_width().padding(8.0))
            .render(ui, dispatch);
    });
}
```

### Пример 2: многострочный комментарий

```rust
TextEdit::new(&state.comment)
    .hint("Введите комментарий...")
    .multiline()
    .max_lines(5)
    .on_change_msg(|v| Msg::CommentChanged(v))
    .render(ui, dispatch);
```

### Пример 3: поиск

```rust
TextEdit::new(&state.query)
    .hint("Поиск")
    .single_line()
    .ime_action(ImeAction::Search)
    .on_change_msg(|v| Msg::QueryChanged(v))
    .on_submit(|q| dispatch.dispatch(Msg::Search(q.to_owned())))
    .render(ui, dispatch);
```

---

## 13. Чеклист приёмки

- [ ] `TextEdit` рендерится без паники в singleline и multiline
- [ ] `hint` отображается когда поле пустое
- [ ] `password(true)` маскирует символы
- [ ] `read_only(true)` запрещает редактирование
- [ ] `char_limit` ограничивает ввод
- [ ] `max_lines` ограничивает высоту multiline
- [ ] `on_changed` вызывается при каждом изменении
- [ ] `on_change_msg` диспатчит Message при каждом изменении
- [ ] `on_submit` вызывается при Done / потере фокуса
- [ ] Клавиатура открывается при фокусе (если KeyboardController зарегистрирован)
- [ ] Клавиатура закрывается при потере фокуса
- [ ] Read-only не открывает клавиатуру
- [ ] Виджет работает внутри Column, Row, Stack, LazyColumn
- [ ] Виджет совместим с Modifier (padding, background, fill_max_width и т.д.)
- [ ] Все unit-тесты проходят
- [ ] Нет изменения файлов в `platform-android`, `core`, `runtime`, `patches/egui`
- [ ] Комментарии на русском
- [ ] Нет `OnceLock`, `std::process::exit(0)`, polling, `Vec<Message>` из View

---

## 14. Зависимости

`crates/ui/Cargo.toml` — **не менять**. Все зависимости уже есть:

- `egui` (workspace) — для `egui::TextEdit`
- `egui-android-core` — для `Widget`, `UiWrapper`
- `egui-android-runtime` — для `Dispatcher`

---

## 15. Что НЕ делать

1. **Не создавать** новый крейт.
2. **Не менять** `patches/egui/` — `egui::TextEdit` уже полностью функционален.
3. **Не менять** `platform-android` для клавиатуры — callback регистрируется через `egui::Context` data.
4. **Не использовать** `ScrollArea` для multiline — egui TextEdit скроллит сам.
5. **Не реализовывать** кастомные маски (P2), автодополнение, контекстное меню.
6. **Не делать** `TextEdit` generic по `&mut String` — значение приходит как `&str` из State.
7. **Не хранить** mutable state внутри виджета между кадрами — только `remember()` или State.



I'll load the task-evaluation-full skill and read the text_edit.md file fully to understand the task, then validate feasibility.

Now let me read the full text_edit.md file to understand the complete task specifications.

Let me read the file in full, section by section.

I now have the full task specification. Let me analyze the feasibility. This is a task-evaluation validation, so I need to check whether the task can be implemented. Let me examine a few critical feasibility concerns:

1. **`KeyboardController` registration** - the spec says `platform-android` should register it, but also says "НЕ изменять `crates/platform-android/*`". This is a conflict I need to check. Let me look at how the platform handles keyboard, IME, and focus currently.

2. **egui `TextEdit` capabilities** - check `password`, `hint`, `char_limit`, `desired_rows`, `interactive`, `gained_focus`/`lost_focus`.

Let me investigate the platform-android IME handling and the egui TextEdit API.

I found a **critical feasibility conflict**. The spec says (Section 15, item 3):
> "Не менять `platform-android` для клавиатуры — callback регистрируется через `egui::Context` data."

But Section 6 says the `KeyboardController` needs to be **registered by platform-android**:
> "Регистрация (platform-android, при инициализации)" with code for `run.rs` or `GlBackend::init`.

So the spec internally contradicts itself: it says don't touch `platform-android`, but also says platform-android must register the keyboard callback. Let me check the current state of platform-android to understand how keyboard is currently triggered (if at all) and whether the `egui::Context` data registration can be cleanly added without broad changes.

Let me look at the run loop and how `show_keyboard` is currently called, plus how focus/keys flow.

Interesting — `show_keyboard`/`hide_keyboard` exist on the backend but are **never invoked** from anywhere in the main flow (only referenced by `process_back_pressed` which calls `backend.hide_keyboard()` when IME is visible). Let me check `Application::show_keyboard`/`hide_keyboard` and how the current IME visibility detection works, and whether there's any wiring that would open the keyboard on focus.

Now I understand the current state. The `show_keyboard`/`hide_keyboard` backend methods exist, and `Application` has `is_keyboard_visible`/`hide_keyboard`. Here's the critical architectural finding:

**There is a structural gap:** The `ui` crate wants to trigger `show_keyboard`/`hide_keyboard` on focus change, but:
1. `ui` cannot depend on `platform-android` (DAG constraint).
2. The spec says the `KeyboardController` callback is registered **by platform-android** via `egui::Context` data with `Id("egui_keyboard_controller")`.
3. But there's currently **no registration of such a controller** in platform-android.

Let me verify whether platform-android currently registers anything in `egui::Context::data()`. Let me check `gl_backend.rs` init and how the `egui::Context` is accessible there.

I can see clearly now. The `egui_ctx` is created in `run.rs` at line 93, and this is exactly where the `KeyboardController` would need to be registered. The problem: this is in **`platform-android`**, which the spec says not to modify.

Let me re-read the spec constraints carefully to figure out the intended design. The spec says in **Section 3 (НЕ изменять)**:
> `crates/platform-android/*` — клавиатура уже работает через `BackendEvent::TextInput` → `egui::Event::Text`

But **Section 6** explicitly requires platform-android to register the controller. And the spec says `KeyboardController` `// Регистрируется в platform-android при инициализации.`.

This is a **real internal contradiction** in the task spec. Let me verify the egui patched TextEdit API to confirm the widget-side capabilities (singleline/multiline, password, hint, char_limit, desired_rows, interactive, gained_focus/lost_focus, event_filter for Enter) all exist.

Let me verify the egui `TextEdit` API used by the widget (this is a vendored copy of egui 0.35). Let me find the TextEdit widget source in the patch.

The patch sources only contain the modified files (the patch is partial). Let me check the PATCH_NOTES.md to confirm what's in the patch and whether `egui::TextEdit` is the standard library one.

The patch only touches `input_state` and `scroll_area`; the rest of egui (including `TextEdit`) is the standard upstream egui 0.35. So `egui::TextEdit` builder methods (`singleline`, `multiline`, `password`, `hint_text`, `char_limit`, `desired_rows`, `interactive`) **all exist** in standard egui 0.35 — the spec's assumptions about the widget API are correct.

Now let me verify the response API the widget needs: `gained_focus()`, `lost_focus()`, and how IME action (Enter) can be intercepted via `response.lost_focus()`. This is standard egui. Also, I need to verify how keyboard visibility is currently detected by the Application (`is_keyboard_visible`). Let me look at the `Application` trait's keyboard methods.

Now I have a complete picture of the feasibility. Let me also verify how the existing `Button` and `Text` handle `on_submit`-like key detection and check what mechanism egui provides for the IME action interception. Let me check the current `with_ui` helper in widget_tests.rs to plan tests.

I now have a complete understanding of the task and the codebase. Let me verify one more thing about how egui `TextEdit` handles focus events and Enter key — I need to confirm the approach for IME action. Let me check the test file's structure for the dispatcher testing pattern (how messages are drained and asserted), since the `on_changed_msg` path needs to dispatch and be verified.

I now have everything needed to validate feasibility. Let me summarize my architectural analysis and produce the task-evaluation.

**Key findings on feasibility:**

1. **Widget API (egui::TextEdit)** — ✅ feasible. All required builder methods exist in standard egui 0.35: `singleline()`, `multiline()`, `password(bool)`, `hint_text(...)`, `char_limit(n)`, `desired_rows(n)`, `interactive(bool)`. The patch only touches `input_state` and `scroll_area` — `TextEdit` is untouched upstream.

2. **Focus/response API** — ✅ feasible. `Response::gained_focus()` / `lost_focus()` / `changed()` are standard egui. BUT the `changed()` event semantics need care: `response.changed()` fires when the text edits. The spec's `on_changed`/`on_changed_msg` can hook this.

3. **The IME action (Enter/Done/Next/Go)** — ⚠️ **This is the largest feasibility gap.** The spec proposees intercepting Enter via `response.lost_focus()`, but this is incomplete/incorrect for a real implementation:
   - `KeyboardType` and `ImeAction` are **UI-layer enums**, but Android's IME action is configured through the platform (the IME's action button label/behavior on the Java side). The spec defines these enums and expects them to control the IME button — but there's **no path** to forward `keyboard_type`/`ime_action` from the `ui` crate to platform-android (DAG forbids `ui → platform-android`).
   - The spec acknowledges this by reducing `ImeAction` handling to just "hide keyboard" for singleline Enter — which is a **reasonable P0 narrowing** but means `keyboard_type`/`ime_action` will be **declared in the API but effectively no-ops** until P2 wiring.
   - Intercepting Enter in singleline: egui `TextEdit::singleline()` already treats Enter as submit/lost-focus by default. The correct hook is `response.lost_focus()` combined with an `event_filter` / checking for the Enter key, or relying on egui's own submit behavior. This is doable.

4. **The core contradiction — `KeyboardController` registration** — 🔴 **Must be resolved with the user.**

## 📋 Оценка задачи

**Глобально (зачем):**
- **Пользователь:** получает возможность вводить текст (login, комментарии, поиск) в Rust-приложениях на Android; клавиатура автоматически появляется при фокусе и скрывается при потере фокуса/по Done.
- **Разработчик:** получает виджет `TextEdit<M>`, интегрированный в MVI (`.on_changed_msg(|v| Msg::X(v))` → dispatch → `store.update`), взамен ручного управления `egui::TextEdit` и IME.

**Задача (что):** Создать виджет `TextEdit<M>` в `crates/ui/src/widgets/text_edit.rs` — обёртку над `egui::TextEdit` с фокус-управлением клавиатурой, MVI-callback и набором builder-опций, плюс unit-тесты в `widget_tests.rs`.

**Критерии успеха:**
- [ ] `TextEdit` рендерится без паники в singleline/multiline, работает в Column/Row/Stack/LazyColumn
- [ ] `hint`, `password`, `read_only`, `char_limit`, `max_lines`, `keyboard_type`, `ime_action` — все builder-методы работают (layout-эффекты)
- [ ] `on_changed` и `on_changed_msg` вызываются при изменении текста (оба — по приоритету §8)
- [ ] `on_submit` + скрытие клавиатуры при Done / потере фокуса
- [ ] Клавиатура открывается на gained_focus, скрывается на lost_focus; **без паники когда контроллер не зарегистрирован**
- [ ] 15 unit-тестов из §11 в `crates/ui/tests/widget_tests.rs`; `cargo test -p egui-android-ui` зелёный
- [ ] Не изменены `core`, `runtime`, `patches/egui`

**Границы (не делаем):**
- ❌ Кастомные маски (P2), автодополнение, контекстное меню
- ❌ `Vec<Message>` из View, `OnceLock` для Sender, polling, `process::exit(0)`
- ❌ Новый крейт; изменения `cargo`-зависимостей
- ❌ `TextEdit` generic по `&mut String`

---

## 🔴 Критический вопрос перед началом (обязательно к согласованию)

Спека **внутренне противоречива** в части клавиатуры (§3 vs §6):

- **§3 (НЕ изменять):** «`crates/platform-android/*` — клавиатура уже работает через `BackendEvent::TextInput`».
- **§6:** «Регистрация (platform-android, при инициализации): `run.rs` / `GlBackend::init` вставляет `KeyboardController` в `egui::Context::data()`» и «`platform-android` заполняет callback».
- **Реальность:** сейчас `show_keyboard`/`hide_keyboard` на бэкенде **существуют, но нигде автоматически не вызываются** по фокусу. `KeyboardController` в `egui::Context::data()` **нигде не зарегистрирован**. `ui` не может импортировать `platform-android` (DAG), значит мост через `Context::data()` — единственный корректный способ, но **без правки `platform-android` (минимум ~5 строк в `run.rs`) клавиатура открываться не будет** — виджет будет «красивой заглушкой».

**Возможные варианты — выберите один:**

| Вариант | Что делаем | Клавиатура по фокусу | Затрагивает platform-android |
|---|---|---|---|
| **A** (рекомендую) | `KeyboardController` определяется в `ui`, регистрируется бэкендом в `run.rs`; widget вызывает (`gained_focus`/`lost_focus`); если не зарегистрирован — пропуск без паники | ✅ работает | Да, ~5 строк в `run.rs` |
| **B** | Только API виджета; `keyboard_type`/`ime_action`/фокус-клавиатура — no-op (P2). `KeyboardController` вообще не регистрируется, widget просто читает `Context::data()` (если кто-то зарегистрирует) | ❌ не в P0 | Нет |
| **C** | Прокинуть клавиатуру через существующий `Application::show_keyboard()/hide_keyboard()` из runtime (не ломая DAG), и вызвать их из frame'а по фокусу | ✅ | Да, но в `runtime` + `Application` trait (затрагивает `runtime`, что §3 тоже запрещает) |

---

**План (порядок — от наиболее инвазивного к наименее):**

| # | Шаг | Инвазивность | Тесты |
|---|-----|--------------|-------|
| 1 | **Согласовать решение по клавиатуре** (A/B/C) → если A: регистрация `KeyboardController` в `run.rs` | 🟠 | — |
| 2 | `text_edit.rs`: enums `KeyboardType`/`ImeAction`, структура `TextEdit<M>`, builder-методы, `KeyboardController` + Id-константа (§6) | 🟡 | юнит (builder config) |
| 3 | `Widget<M> for TextEdit<M>`: render через `ui.add(egui::TextEdit)`, применять опции, обрабатывать gained/lost focus, changed, Enter/lost_focus→submit | 🟡 | юнит + интеграционные |
| 4 | Подключение: `widgets/mod.rs` `mod text_edit;` + `pub use`, `lib.rs` в `pub use widgets` | 🟢 | — |
| 5 | **15 тестов** из §11 в `widget_tests.rs` (с паттерном `with_ui` + проверкой dispatch через `Dispatcher::try_iter`) | 🟢 | юнит |
| 6 | `cargo test -p egui-android-ui` (+ `cargo check --workspace`) | 🟢 | — |

---

**Резюме валидации:** задача **реализуема**, виджет-часть (`egui::TextEdit` API, focus-события, MVI-callback) полностью поддерживается текущим egui 0.35 и архитектурой. Единственный блокер — регистрация `KeyboardController`: без пункта в плане (правка `platform-android/run.rs`) клавиатура по фокусу работать не будет, несмотря на §3. Нужно ваше решение по варианту **A/B/C** прежде чем начинать.

Какой вариант выбираете?

A Вариант выбрали
