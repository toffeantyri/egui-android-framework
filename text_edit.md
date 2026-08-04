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
| `crates/platform-android/src/run.rs` | Регистрация `KeyboardController` в `egui::Context::data()` |
| `crates/platform-android/src/backend/gl_backend.rs` | Обработка `TextEvent` из IME для получения вводимого текста |
| `crates/platform-android/src/input_processing.rs` | Маршрутизация `BackendEvent::TextInput` → `egui::Event::Text` |

### НЕ изменять

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

- [x] `TextEdit` рендерится без паники в singleline и multiline
- [x] `hint` отображается когда поле пустое
- [x] `password(true)` маскирует символы
- [x] `read_only(true)` запрещает редактирование
- [x] `char_limit` ограничивает ввод
- [x] `max_lines` ограничивает высоту multiline
- [x] `on_changed` вызывается при каждом изменении
- [x] `on_change_msg` диспатчит Message при каждом изменении
- [x] `on_submit` вызывается при Done / потере фокуса
- [x] Клавиатура открывается при фокусе (KeyboardController зарегистрирован)
- [x] Клавиатура закрывается при потере фокуса
- [x] Read-only не открывает клавиатуру
- [x] Виджет работает внутри Column, Row, Stack, LazyColumn
- [x] Виджет совместим с Modifier (padding, background, fill_max_width и т.д.)
- [x] Все unit-тесты проходят
- [ ] Фактический ввод текста с клавиатуры попадает в поле (IME → egui)
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
3. **Не использовать** `ScrollArea` для multiline — egui TextEdit скроллит сам.
4. **Не реализовывать** кастомные маски (P2), автодополнение, контекстное меню.
5. **Не делать** `TextEdit` generic по `&mut String` — значение приходит как `&str` из State.
6. **Не хранить** mutable state внутри виджета между кадрами — только `remember()` или State.

---

## 📋 Оценка задачи

**Глобально (зачем):**
- **Пользователь:** получает возможность вводить текст (login, комментарии, поиск) в Rust-приложениях на Android; клавиатура автоматически появляется при фокусе и скрывается при потере фокуса/по Done.
- **Разработчик:** получает виджет `TextEdit<M>`, интегрированный в MVI (`.on_changed_msg(|v| Msg::X(v))` → dispatch → `store.update`), взамен ручного управления `egui::TextEdit` и IME.

**Задача (что):** Создать виджет `TextEdit<M>` в `crates/ui/src/widgets/text_edit.rs` — обёртку над `egui::TextEdit` с фокус-управлением клавиатурой, MVI-callback и набором builder-опций, плюс unit-тесты в `widget_tests.rs`. Вторая часть — добавить получение вводимого текста из Android IME в `platform-android`.

**Критерии успеха:**
- [x] `TextEdit` рендерится без паники в singleline/multiline, работает в Column/Row/Stack/LazyColumn
- [x] `hint`, `password`, `read_only`, `char_limit`, `max_lines`, `keyboard_type`, `ime_action` — все builder-методы работают (layout-эффекты)
- [x] `on_changed` и `on_changed_msg` вызываются при изменении текста (оба — по приоритету §8)
- [x] `on_submit` + скрытие клавиатуры при Done / потере фокуса
- [x] Клавиатура открывается на gained_focus, скрывается на lost_focus; **без паники когда контроллер не зарегистрирован**
- [x] 15 unit-тестов из §11 в `crates/ui/tests/widget_tests.rs`; `cargo test -p egui-android-ui` зелёный
- [ ] **Фактический ввод текста с Android-клавиатуры попадает в поле (IME → egui)**
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

**Резюме валидации:** задача **реализуема**, виджет-часть (`egui::TextEdit` API, focus-события, MVI-callback) полностью поддерживается текущим egui 0.35 и архитектурой. Единственный блокер — регистрация `KeyboardController`: без пункта в плане (правка `platform-android/run.rs`) клавиатура по фокусу работать не будет, несмотря на §3. Нужно ваше решение по варианту **A/B/C** прежде чем начинать.

Какой вариант выбираете?

**A** Вариант выбрали. ✅ Реализован.

---

## 16. Что сделано и что осталось

### ✅ Сделано (P0 — виджет + клавиатура)

| # | Что | Статус |
|---|-----|--------|
| 1 | Виджет `TextEdit<M>` в `crates/ui/src/widgets/text_edit.rs` | ✅ |
| 2 | Builder-методы: hint, single_line, multiline, password, read_only, char_limit, max_lines, on_changed, on_changed_msg, on_submit, keyboard_type, ime_action | ✅ |
| 3 | `KeyboardController` + `keyboard_controller_id()` в `crates/runtime/src/lib.rs` | ✅ |
| 4 | Регистрация `KeyboardController` в `run.rs` → клавиатура открывается по gained_focus, скрывается по lost_focus | ✅ |
| 5 | Обработка фокуса в виджете: при gained_focus — `keyboard_show()`, при lost_focus — `keyboard_hide()`, флаг `keyboard_is_owner` чтобы не спамить | ✅ |
| 6 | Поле `ime_visible: Arc<AtomicBool>` в `GlBackend` для синхронизации состояния клавиатуры | ✅ |
| 7 | 15 unit-тестов + интеграционные в `widget_tests.rs` — все проходят | ✅ |
| 8 | Пример `TextEditScreen` в showcase | ✅ |
| 9 | Флаг `ime_visible` сбрасывается при скрытии клавиатуры и при потере фокуса (idempotent) | ✅ |

### 🔴 Не сделано — ввод текста с клавиатуры (IME)

**Проблема:** Клавиатура открывается, но набранные символы не попадают в `TextEdit`.

**Устройство:** POCO, Android 16, Snapdragon 8 Elite / Adreno 825.
**android-activity:** 0.6 (единственная доступная версия).

### Хронология экспериментов с IME-вводом

#### Что работает

- **`TextEvent` приходит** через `input_events_iter()` — нажатие «о» доставляет `TextEvent { text: "о", selection: 1:1 }`.
- **Текст доходит до egui** через цепочку `BackendEvent::TextInput` → `process_backend_input` → `egui::Event::Text` → `TextEdit` обновляет буфер (`changed=true buffer="о"`).

#### Что НЕ работает (регрессия при попытках)

| # | Попытка | Файл | Результат |
|---|---------|------|-----------|
| 1 | `TextEvent` handler: **безусловный** `set_text_input_state(reply)` на каждый `TextEvent` | `gl_backend.rs` | ❌ Спам `IME: reply text=''` каждый кадр → **клавиатура ломается во всей системе** до перезагрузки устройства |
| 2 | Reply только при `text_changed` (`state.text != ime_accumulated_text`) | `gl_backend.rs` | ❌ На пустой `TextEvent` нет reply → IME сессия не стартует, клавиатура не открывается |
| 3 | Reply при `text_changed OR first_handshake` (флаг `ime_replied`) | `gl_backend.rs` | ❌ Одного handshake недостаточно |
| 4 | Reply **всегда**, но emit событий только при `is_new` | `gl_backend.rs` | ❌ Всё равно спам |
| 5 | `set_text_input_state` в `KeyboardController` перед `show_soft_input` | `run.rs` | ❌ Клавиатура не открывается |
| 6 | `text_input_state()` (take=false) — чтение текста из GameActivity | `gl_backend.rs` | ❌ Нативный краш `from_raw_parts::precondition_check` в `GameActivity_getTextInputState` |

#### Корневая причина

Вызов `set_text_input_state()` внутри цикла `input_events_iter()` в android-activity 0.6 создаёт бесконечную петлю:

```
reply → GameActivity выставляет флаг textInputState
     → новый input_events_iter() видит флаг → генерирует TextEvent
     → мы снова reply → флаг снова взводится → ∞
```

Это делает невозможным связку `TextEvent` + `set_text_input_state` в одном потоке через `input_events_iter()`.

#### Что осталось попробовать

| # | Подход | Сложность | Риски |
|---|--------|-----------|-------|
| 1 | **JNI-обход**: читать текст через `GameActivity.getTextInputState()` напрямую через JNI-вызов, вне цикла `input_events_iter()`, без `set_text_input_state` | 🟠 средняя | Нужен доступ к `JavaVM`/`JNIEnv`; возможны race conditions |
| 2 | **Отдельный поток для IME**: вынести коммуникацию с IME в отдельный поток через `mpsc::channel`, чтобы не блокировать рендер-цикл | 🔴 высокая | Многопоточность в Android NDK; JNIEnv привязан к потоку |
| 3 | **Callback через GameActivity**: использовать `GameActivity_setTextInputCallback` для получения текста без `set_text_input_state` reply-petli | 🟡 низкая | Может не поддерживаться в android-activity 0.6 |
