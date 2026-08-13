//! Виджет [`TextEdit`] — обёртка над `egui::TextEdit`, интегрированная в MVI.
//!
//! Принимает значение `value: &str` (из State или `remember`), при изменении
//! текста вызывает пользовательский callback (замыкание или Message), при
//! смене фокуса управляет клавиатурой (IME) через `KeyboardController`
//! из `egui::Context::data()`.
//!
//! Референс поведения — `TextField` / `BasicTextField` из Jetpack Compose.
//!
//! # Архитектура
//!
//! - Виджет — чистая декларация: не читает State напрямую, не вызывает Store.
//!   Значение приходит через `&str`, изменение передаётся наружу через
//!   callback (`.on_changed` / `.on_change_msg` / `.on_submit`).
//! - Сообщение диспатчится через `Dispatcher<M>` в момент события (callback).
//! - Клавиатура управляется по событию фокуса (push), без polling.
//!
//! # Пример
//!
//! ```ignore
//! TextEdit::new(&state.email)
//!     .hint("Email")
//!     .single_line()
//!     .keyboard_type(KeyboardType::Email)
//!     .on_change_msg(|v| Msg::EmailChanged(v))
//!     .on_submit(|v| dispatch.dispatch(Msg::Submit(v.to_owned())))
//!     .render(ui, dispatch);
//! ```

use std::sync::{Arc, RwLock};

use egui_android_core::{widget::Widget, UiWrapper};
use egui_android_runtime::{
    keyboard_controller_id, Dispatcher, ImeEditorState, KeyboardController,
};

/// Тип клавиатуры для IME.
///
/// В P0 тип хранится в виджете и доступен для проверки/тестов. Накопление
/// IME inputType на Android требует платформенного моста (P2) — в текущей
/// реализации тип не влияет на поведение клавиатуры.
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
///
/// В P0 все действия обрабатываются одинаково (singleline: Enter → submit) —
/// как указано в контракте. Передача фокуса на следующее поле (Next/Focus)
/// — P2.
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
/// # MVI-паттерны
///
/// **Паттерн 1 — полный MVI** (каждый символ в Store):
/// ```ignore
/// TextEdit::new(&state.email)
///     .on_change_msg(|v| Msg::EmailChanged(v))
///     .render(ui, dispatch);
/// ```
///
/// **Паттерн 2 — локальный `remember` + submit**:
///
/// ⚠️ Read-guard от `local.get()` нужно разорвать ДО `render`, иначе:
/// `TextEdit::new(local.get().clone()).on_changed(move |v| local.set(v))...render()`
/// в одном полном выражении оставляет временный `RwLockReadGuard` живым на время
/// `render()`, а `on_changed -> local.set` берёт write на тот же `std::sync::RwLock`
/// -> самоблокировка (self-deadlock). Безопасно:
///
/// ```ignore
/// let local = remember(ui, "email_input", || String::new());
/// let init = local.get().clone();   // guard уничтожен здесь
/// let local = local.clone();
/// TextEdit::new(init)
///     .on_changed(move |v| local.set(v.to_owned()))
///     .on_submit(move |v| dispatch.dispatch(Msg::Submit(v.to_owned())))
///     .render(ui, dispatch);
/// ```
///
/// **Паттерн 3 — кастомная логика**:
/// ```ignore
/// TextEdit::new(&state.phone)
///     .on_changed(move |v| {
///         if v.len() <= 18 {
///             dispatch.dispatch(Msg::PhoneChanged(v.to_owned()));
///         }
///     })
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
    on_changed: Option<Arc<dyn Fn(&str) + Send + Sync>>,

    /// Альтернатива on_changed: замыкание, формирующее Message
    /// из нового значения для dispatch в Store.
    on_changed_msg: Option<Arc<dyn Fn(String) -> M + Send + Sync>>,

    /// Вызывается при IME action (Done/Search) или потере фокуса.
    /// По умолчанию Done → скрыть клавиатуру.
    on_submit: Option<Arc<dyn Fn(&str) + Send + Sync>>,

    /// Тип клавиатуры.
    keyboard_type: KeyboardType,

    /// Действие кнопки IME.
    ime_action: ImeAction,

    /// Внутренний отступ (margin) поля — пространство между текстом и рамкой/фоном
    /// поля. Внешний отступ (spacing между виджетами) задаётся модификатором
    /// `Modifier.padding(...)`. По умолчанию — комфортный запас по обеим осям.
    internal_padding: f32,

    /// Явный id поля (стабильный между кадрами). Если `None` — id берётся
    /// автоматически (`ui.next_auto_id()`). Явный id полезен для устойчивости
    /// буфера ввода при изменении структуры UI. Буфер и само поле используют
    /// один и тот же id.
    field_id: Option<egui::Id>,
}

impl<M: 'static> TextEdit<M> {
    /// Создать новый TextEdit с начальным значением.
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            hint_text: String::new(),
            single_line: true,
            password: false,
            max_lines: None,
            char_limit: None,
            read_only: false,
            on_changed: None,
            on_changed_msg: None,
            on_submit: None,
            keyboard_type: KeyboardType::Text,
            ime_action: ImeAction::Done,
            internal_padding: 6.0,
            field_id: None,
        }
    }

    /// Текст-подсказка (placeholder).
    pub fn hint(mut self, text: impl Into<String>) -> Self {
        self.hint_text = text.into();
        self
    }

    /// Однострочный режим (по умолчанию).
    pub fn single_line(mut self) -> Self {
        self.single_line = true;
        self
    }

    /// Многострочный режим.
    pub fn multiline(mut self) -> Self {
        self.single_line = false;
        self
    }

    /// Маска пароля.
    pub fn password(mut self) -> Self {
        self.password = true;
        self
    }

    /// Максимум строк (multiline).
    pub fn max_lines(mut self, n: usize) -> Self {
        self.max_lines = Some(n);
        self
    }

    /// Лимит символов.
    pub fn char_limit(mut self, n: usize) -> Self {
        self.char_limit = Some(n);
        self
    }

    /// Только чтение (без редактирования).
    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }

    /// Замыкание при изменении (локальная логика).
    pub fn on_changed<F: Fn(&str) + Send + Sync + 'static>(mut self, f: F) -> Self {
        self.on_changed = Some(Arc::new(f));
        self
    }

    /// Замыкание, формирующее Message при изменении (MVI-поток).
    pub fn on_change_msg<F: Fn(String) -> M + Send + Sync + 'static>(mut self, f: F) -> Self {
        self.on_changed_msg = Some(Arc::new(f));
        self
    }

    /// Замыкание при submit (Done / потеря фокуса).
    pub fn on_submit<F: Fn(&str) + Send + Sync + 'static>(mut self, f: F) -> Self {
        self.on_submit = Some(Arc::new(f));
        self
    }

    /// Тип клавиатуры.
    pub fn keyboard_type(mut self, kt: KeyboardType) -> Self {
        self.keyboard_type = kt;
        self
    }

    /// Действие кнопки IME.
    pub fn ime_action(mut self, action: ImeAction) -> Self {
        self.ime_action = action;
        self
    }

    /// Задать внутренний отступ поля (margin) в точках.
    ///
    /// Это пространство между текстом и рамкой/фоном поля. По умолчанию — 6.0.
    /// Внешний отступ между виджетами задаётся через `Modifier.padding(...)`
    /// (см. [`crate::modifier::Modifier::padding`]).
    pub fn internal_padding(mut self, pad: f32) -> Self {
        self.internal_padding = pad;
        self
    }

    /// Задать явный стабильный id поля.
    ///
    /// Полезно, когда порядок/структура виджетов может меняться (в `LazyColumn`,
    /// обёртках), а нужно, чтобы буфер введённого текста не сбрасывался.
    /// Буфер и само поле используют один и тот же id. По умолчанию id
    /// выбирается автоматически (`ui.next_auto_id()`).
    pub fn id(mut self, id: egui::Id) -> Self {
        self.field_id = Some(id);
        self
    }

    /// Текущий внутренний отступ (для тестов).
    pub fn get_internal_padding(&self) -> f32 {
        self.internal_padding
    }

    /// Текущее значение (для тестов и отладки).
    pub fn get_value(&self) -> &str {
        &self.value
    }

    /// Признак single-line режима (для тестов).
    pub fn is_single_line(&self) -> bool {
        self.single_line
    }

    /// Признак read-only (для тестов).
    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    /// Признак password-режима (для тестов).
    pub fn is_password(&self) -> bool {
        self.password
    }

    /// Максимум строк (для тестов).
    pub fn get_max_lines(&self) -> Option<usize> {
        self.max_lines
    }

    /// Выбранный KeyboardType (для тестов).
    pub fn get_keyboard_type(&self) -> KeyboardType {
        self.keyboard_type
    }

    /// Выбранный ImeAction (для тестов).
    pub fn get_ime_action(&self) -> ImeAction {
        self.ime_action
    }
}

impl<M: Send + 'static> Widget<M> for TextEdit<M> {
    fn render(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<M>) {
        // Контролируемый буфер.
        //
        // `egui::TextEdit` вставляет введённое прямо в переданный `&mut String`,
        // и это значение нужно хранить стабильно между кадрами (иначе текст
        // «пропадает»). Буфер теперь хранится в `KeyboardController.text_buffers`
        // (`Arc<Mutex<HashMap<Id, Arc<RwLock<String>>>>>`), чтобы инициализация
        // НЕ вызывала `ctx.data_mut` внутри render (reentrant write-lock на
        // Context -> deadlock).
        let field_id = self.field_id.unwrap_or_else(|| ui.next_auto_id());

        let buffer_arc: Arc<RwLock<String>> = ui
            .ctx()
            .data(|d| {
                d.get_temp::<KeyboardController>(keyboard_controller_id())
                    .map(|kb| kb.text_buffer(field_id))
            })
            .unwrap_or_else(|| {
                // Если контроллер не зарегистрирован (десктоп/тесты) — создаём
                // буфер локально (не в контексте).
                Arc::new(RwLock::new(self.value.clone()))
            });

        let mut text_guard = buffer_arc.write().expect("TextEdit: буфер poisoned");

        // При первом создании буфер пуст — записываем начальное значение
        // виджета (если задано). Это заменяет старую инициализацию через
        // `ctx.data_mut`.
        if text_guard.is_empty() && !self.value.is_empty() {
            *text_guard = self.value.clone();
        }

        let mut te = if self.single_line {
            egui::TextEdit::singleline(&mut *text_guard)
        } else {
            egui::TextEdit::multiline(&mut *text_guard)
        };
        // Фиксируем id поля — тот же, что используется для буфера (см. выше).
        te = te.id(field_id);

        // Применяем настройки.
        if !self.hint_text.is_empty() {
            te = te.hint_text(self.hint_text.as_str());
        }
        if self.password {
            te = te.password(true);
        }
        if let Some(limit) = self.char_limit {
            te = te.char_limit(limit);
        }
        if let Some(rows) = self.max_lines {
            te = te.desired_rows(rows);
        }
        // read_only → интерактивность off (клавиатура не открывается).
        te = te.interactive(!self.read_only);

        // Внутренний отступ поля: пространство между текстом и рамкой/фоном.
        // Внешний отступ между виджетами задаётся модификатором `Modifier.padding(...)`.
        let ip = self.internal_padding.clamp(0.0, 100.0) as i8;
        te = te.margin(egui::Margin::symmetric(ip, ip));

        // Растягивание на ширину родителя: `egui::TextEdit` alloc'ит ширину по
        // контенту (тексту) и игнорирует `min_width` constraints. Чтобы `fill_max_width`
        // заставил поле занять всю ширину, явно передаём желаемую ширину,
        // когда родитель требует min_width > 0.
        let min_w = ui.constraints().min_width;
        if min_w > 0.0 {
            te = te.desired_width(min_w);
        }

        let response = ui.add(te);

        // Логирование для локализации проблемы ввода/фокуса (временная диагностика).
        if response.gained_focus() || response.lost_focus() || response.changed() {
            log::info!(
                "[TextEdit] id={:?} has_focus={} gained={} lost={} changed={} buffer={:?}",
                field_id,
                response.has_focus(),
                response.gained_focus(),
                response.lost_focus(),
                response.changed(),
                &*text_guard
            );
        }

        // ─── Управление клавиатурой (по событию фокуса) ───
        //
        // Проблема: `lost_focus` в egui задерживается на 1-2 кадра с момента
        // перевода фокуса (см. `Memory::lost_focus` и тест `lost_focus_fires_after_mid_frame_focus_transfer`).
        // При тапе на другой TextEdit: новый editor получает `gained_focus` в кадре N,
        // а старый — `lost_focus` только в кадре N+1. Если прятать клавиатуру по `lost_focus`
        // мгновенно, то кадр N+1 (когда новый editor уже не стреляет `gained_focus`, а старый
        // ещё стреляет `lost_focus`) закроет клавиатуру и не откроет её заново.
        //
        // Решение: держим "владельца" клавиатуры (Id фокусного редактора) в общем состоянии
        // `Context::data()`. Клавиатуру прячем ТОЛЬКО когда фокус теряет сам владелец,
        // а не когда кто-то ещё стреляет `lost_focus`.
        if !self.read_only {
            let is_focused = response.has_focus();
            // Регистрируем поле в упорядоченном реестре (для IME_ACTION_NEXT),
            // независимо от фокуса — порядок отрисовки сохраняется между кадрами.
            ime_field_register(ui, field_id);

            if is_focused {
                // Я — фокусный редактируемый editor. Если ещё не я владею клавиатурой
                // (вызов idempotent по кадрам, `show_soft_input` не спамится),
                // становимся владельцем, обновляем EditorInfo и показываем клавиатуру.
                if !keyboard_is_owner(ui, field_id) {
                    keyboard_set_options(ui, self.keyboard_type, self.ime_action);
                    keyboard_set_owner(ui, field_id);
                    keyboard_show(ui);
                }
                // Пока поле в фокусе — публикуем состояние (текст + курсор в
                // UTF-16) в платформу для двустороннего InputConnection.
                keyboard_publish_editor_state(ui, field_id, &*text_guard);
            } else if response.lost_focus() {
                // Я потерял фокус. Прячем клавиатуру ТОЛЬКО если я был её владельцем.
                // Если фокус ушёл на другой TextEdit — он уже стал владельцем (или станет
                // в этот же кадр), и это условие не сработает.
                if keyboard_is_owner(ui, field_id) {
                    keyboard_publish_editor_blur(ui);
                    keyboard_hide(ui);
                    keyboard_clear_owner(ui);
                }
            }
        }

        // Снимаем write-lock с буфера — теперь он только читается в changed/submit.
        drop(text_guard);

        // ─── Изменение текста → callback ───
        if response.changed() {
            // write-lock уже снят (drop выше). Читаем изменённое значение через read.
            let new_text: String = buffer_arc.read().expect("TextEdit: буфер poisoned").clone();
            // log::info!("[TextEdit] enter changed-block, buffer={:?}", &new_text);
            // log::info!("[TextEdit] buffer cloned: {:?}", &new_text);
            // Приоритет: сначала локальный on_changed, затем on_change_msg.
            if let Some(cb) = &self.on_changed {
                // log::info!("[TextEdit] calling on_changed");
                cb(&new_text);
                // log::info!("[TextEdit] on_changed done");
            }
            if let Some(cb) = &self.on_changed_msg {
                // log::info!("[TextEdit] calling on_change_msg/dispatch");
                dispatch.dispatch(cb(new_text));
                // log::info!("[TextEdit] on_change_msg done");
            }
            // log::info!("[TextEdit] changed-block done");
        }

        // ─── Submit (Done / потеря фокуса) — использует text_guard, которого
        // может уже не быть; перечитываем через блокирующий read. ───
        if response.lost_focus() {
            let snapshot = buffer_arc.read().expect("TextEdit: буфер poisoned").clone();
            if let Some(cb) = &self.on_submit {
                cb(&snapshot);
            }
        }
        // log::info!("[TextEdit] render end"); // спам-лог, закомментирован
    }
}

/// Показать клавиатуру через `KeyboardController` (если зарегистрирован).
///
/// Если контроллер не зарегистрирован (десктоп, тесты, NativeBackend без IME)
/// — просто пропускаем, без паники.
fn keyboard_show(ui: &UiWrapper) {
    ui.ctx().data(|d| {
        if let Some(kb) = d.get_temp::<KeyboardController>(keyboard_controller_id()) {
            kb.show();
        }
    });
}

/// Скрыть клавиатуру через `KeyboardController` (если зарегистрирован).
fn keyboard_hide(ui: &UiWrapper) {
    ui.ctx().data(|d| {
        if let Some(kb) = d.get_temp::<KeyboardController>(keyboard_controller_id()) {
            kb.hide();
        }
    });
}

/// Передать `inputType` + `imeOptions` текущего поля в IME (EditorInfo),
/// если `KeyboardController` поддерживает настройку (платформа-регистратор).
fn keyboard_set_options(ui: &UiWrapper, keyboard_type: KeyboardType, ime_action: ImeAction) {
    // log::info!("[TextEdit] set_options enter"); // спам
    ui.ctx().data(|d| {
        if let Some(kb) = d.get_temp::<KeyboardController>(keyboard_controller_id()) {
            let input_type = android_input_type(keyboard_type);
            let ime_options = android_ime_options(ime_action);
            kb.set_options(input_type, ime_options);
        }
    });
    // log::info!("[TextEdit] set_options exit"); // спам
}

/// Число UTF-16 code units в первых `n` символах строки.
///
/// Android `InputConnection` индексирует текст в UTF-16 code units, тогда как
/// egui-курсор — в символах (`CharIndex`).
fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

/// Позиция в UTF-16 code units для символьного индекса `char_index`.
#[allow(dead_code)]
fn utf16_char_index(text: &str, char_index: usize) -> usize {
    text.chars().take(char_index).map(|c| c.len_utf16()).sum()
}

/// Публикация состояния редактирования фокусного поля в платформу
/// (двусторонний `InputConnection`).
///
/// Если `KeyboardController` привязал слот `ImeEditorStateSlot` (платформа),
/// записываем позицию курсора/выделения в UTF-16 code units. Если поля
/// не имеют egui-курсора (None) — помещаем курсор в конец текста.
fn keyboard_publish_editor_state(ui: &UiWrapper, _field_id: egui::Id, text: &str) {
    // log::info!("[TextEdit] publish_editor_state enter {:?}", field_id); // спам
    let slot = ui.ctx().data(|d| {
        d.get_temp::<KeyboardController>(keyboard_controller_id())
            .and_then(|kb| kb.editor_state().cloned())
    });
    let Some(slot) = slot else {
        return; // платформа не привязала двусторонний канал
    };

    let text_len = utf16_len(text);
    // Позиция курсора для InputConnection. Во время активного IME-ввода egui в
    // `TextEditState.cursor.char_range()` хранит preedit-композицию как ВЫДЕЛЕНИЕ
    // от начала (например `0..2` для предикта «пр»), а не каретку в конце. Если
    // публиковать это выделение как `selection_start=0`, Gboard решает, что курсор
    // в начале текста, шлёт setComposingRegion от начала и накатывает предикт ПОВЕРХ
    // уже набранного — ввод заменяется (баг «привет → ет»). Поэтому для IME всегда
    // публикуем каретку в КОНЦЕ текста (как обычный EditText).
    let selection_end = text_len;
    let selection_start = text_len;

    let state = ImeEditorState {
        focused: true,
        text: text.to_owned(),
        text_len,
        selection_start,
        selection_end,
        composing_start: None,
        composing_end: None,
    };
    *slot.lock().unwrap() = Some(state);
    // log::info!("[TextEdit] publish_editor_state exit {:?}", field_id); // спам
}

/// Сбросить состояние редактора (поле потеряло фокус): IME/InputConnection
/// больше не должен отдавать текст/курсор.
fn keyboard_publish_editor_blur(ui: &UiWrapper) {
    // log::info!("[TextEdit] publish_editor_blur enter"); // спам
    let slot = ui.ctx().data(|d| {
        d.get_temp::<KeyboardController>(keyboard_controller_id())
            .and_then(|kb| kb.editor_state().cloned())
    });
    if let Some(slot) = slot {
        *slot.lock().unwrap() = None;
    }
    // log::info!("[TextEdit] publish_editor_blur exit"); // спам
}

// ─── Реестр полей ввода (порядок отрисовки) для IME_ACTION_NEXT ──────────
//
// Упорядоченный список id редактируемых полей хранится в `Context::data()`.
// Каждый кадр, пока поле рендерится, оно обеспечивает своё присутствие в
// списке (без дубликатов). `IME_ACTION_NEXT` переводит фокус к следующему.

/// Зарегистрировать поле в упорядоченном реестре (без дубликатов).
///
/// Реестр хранится в `KeyboardController.registry_slot` (Arc<RwLock>) —
/// мутация без `ctx.data_mut` в render (reentrant write-lock -> dead).
fn ime_field_register(ui: &UiWrapper, field_id: egui::Id) {
    ui.ctx().data(|d| {
        if let Some(kb) = d.get_temp::<KeyboardController>(keyboard_controller_id()) {
            let mut list = kb.registry_slot().write().unwrap();
            if !list.contains(&field_id) {
                list.push(field_id);
            }
        }
    });
}

// ─── Маппинг KeyboardType/ImeAction в битовые маски Android (EditorInfo) ───
//
// Значения совпадают с константами Android SDK:
// - `android.text.InputType` (TYPE_CLASS_*, TYPE_TEXT_VARIATION_*)
// - `android.view.inputmethod.EditorInfo` (IME_ACTION_*, IME_FLAG_*)
// Хранятся здесь (в ui), т.к. `KeyboardType`/`ImeAction` — типы виджета;
// платформа передаёт числа в Kotlin, где они интерпретируются нативно.

/// `InputType` из `KeyboardType` (базовый класс + вариация).
pub(crate) fn android_input_type(kt: KeyboardType) -> i32 {
    use KeyboardType::*;
    // InputType
    const TYPE_CLASS_TEXT: i32 = 0x0000_0001;
    const TYPE_CLASS_NUMBER: i32 = 0x0000_0002;
    const TYPE_CLASS_PHONE: i32 = 0x0000_0003;
    const TYPE_TEXT_VARIATION_EMAIL_ADDRESS: i32 = 0x0000_0020;
    const TYPE_TEXT_VARIATION_URI: i32 = 0x0000_0010;
    const TYPE_TEXT_VARIATION_PASSWORD: i32 = 0x0000_0080;

    match kt {
        Email => TYPE_CLASS_TEXT | TYPE_TEXT_VARIATION_EMAIL_ADDRESS,
        Phone => TYPE_CLASS_PHONE,
        Number => TYPE_CLASS_NUMBER,
        Password => TYPE_CLASS_TEXT | TYPE_TEXT_VARIATION_PASSWORD,
        Uri => TYPE_CLASS_TEXT | TYPE_TEXT_VARIATION_URI,
        Text => TYPE_CLASS_TEXT,
    }
}

/// `imeOptions` (imeAction + флаги) из `ImeAction`.
pub(crate) fn android_ime_options(action: ImeAction) -> i32 {
    use ImeAction::*;
    // EditorInfo.IME_ACTION_*
    const FLAG_NO_EXTRACT_UI: i32 = 0x1000_0000;
    const ACTION_GO: i32 = 0x0000_0002;
    const ACTION_SEARCH: i32 = 0x0000_0003;
    const ACTION_NEXT: i32 = 0x0000_0005;
    const ACTION_DONE: i32 = 0x0000_0006;

    let action = match action {
        Go => ACTION_GO,
        Search => ACTION_SEARCH,
        Next => ACTION_NEXT,
        Done => ACTION_DONE,
    };
    action | FLAG_NO_EXTRACT_UI
}

// Буфер поля теперь хранится в `KeyboardController.text_buffers`.

/// Является ли `id` текущим владельцем клавиатуры.
fn keyboard_is_owner(ui: &UiWrapper, id: egui::Id) -> bool {
    ui.ctx().data(|d| {
        d.get_temp::<KeyboardController>(keyboard_controller_id())
            .map(|kb| *kb.owner_slot().read().unwrap() == Some(id))
            .unwrap_or(false)
    })
}

/// Назначить `id` владельцем клавиатуры.
///
/// Пишем в `owner_slot` внутри контроллера (Arc<RwLock>) — без `ctx.data_mut`
/// в render (reentrant write-lock на Context -> deadlock).
fn keyboard_set_owner(ui: &UiWrapper, id: egui::Id) {
    // log::info!("[TextEdit] set_owner enter {:?}", id); // спам
    ui.ctx().data(|d| {
        if let Some(kb) = d.get_temp::<KeyboardController>(keyboard_controller_id()) {
            *kb.owner_slot().write().unwrap() = Some(id);
        }
    });
    // log::info!("[TextEdit] set_owner exit {:?}", id); // спам
}

/// Сбросить владельца клавиатуры.
fn keyboard_clear_owner(ui: &UiWrapper) {
    // log::info!("[TextEdit] clear_owner enter"); // спам
    ui.ctx().data(|d| {
        if let Some(kb) = d.get_temp::<KeyboardController>(keyboard_controller_id()) {
            *kb.owner_slot().write().unwrap() = None;
        }
    });
    // log::info!("[TextEdit] clear_owner exit"); // спам
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_android_runtime::KeyboardController;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    /// Прогон UI-замыкания в egui Context (как в интеграционных тестах).
    fn with_ui(f: impl FnOnce(&mut UiWrapper)) {
        let f = std::cell::RefCell::new(Some(f));
        let ctx = egui::Context::default();
        let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let f = f.borrow_mut().take().unwrap();
                f(&mut UiWrapper::new_unconstrained(ui));
            });
        });
    }

    /// Регистрация KeyboardController и подсчёт вызовов show/hide.
    fn register_controller(ctx: &egui::Context) -> (Arc<AtomicUsize>, Arc<AtomicUsize>) {
        let show_count = Arc::new(AtomicUsize::new(0));
        let hide_count = Arc::new(AtomicUsize::new(0));
        let sc = Arc::clone(&show_count);
        let hc = Arc::clone(&hide_count);
        let kb = KeyboardController::with_options(
            Arc::new(move || {
                sc.fetch_add(1, Ordering::SeqCst);
            }),
            Arc::new(move || {
                hc.fetch_add(1, Ordering::SeqCst);
            }),
            Arc::new(|_, _| {}),
        );
        ctx.data_mut(|d| {
            d.insert_temp(keyboard_controller_id(), kb);
        });
        (show_count, hide_count)
    }

    /// Регистрация контроллера + захват последних аргументов `set_options`
    /// (inputType, imeOptions) и счётчика вызовов. Нужно для проверки, что при
    /// смене владельца/фокуса поле обновляет EditorInfo (Next→Done и т.п.).
    fn register_controller_with_options(
        ctx: &egui::Context,
    ) -> (Arc<Mutex<Option<(i32, i32)>>>, Arc<AtomicUsize>) {
        let last = Arc::new(Mutex::new(None));
        let count = Arc::new(AtomicUsize::new(0));
        let last_b = Arc::clone(&last);
        let count_b = Arc::clone(&count);
        let kb = KeyboardController::with_options(
            Arc::new(|| {}),
            Arc::new(|| {}),
            Arc::new(move |input_type, ime_options| {
                *last_b.lock().unwrap() = Some((input_type, ime_options));
                count_b.fetch_add(1, Ordering::SeqCst);
            }),
        );
        ctx.data_mut(|d| {
            d.insert_temp(keyboard_controller_id(), kb);
        });
        (last, count)
    }

    #[test]
    fn owner_tracking_set_and_is_owner() {
        with_ui(|ui| {
            let _ = register_controller(ui.ctx());
            let id_a = egui::Id::new("editor_a");
            assert!(!keyboard_is_owner(ui, id_a), "изначально никто не владеет");
            keyboard_set_owner(ui, id_a);
            assert!(keyboard_is_owner(ui, id_a), "A должен стать владельцем");
            assert!(
                !keyboard_is_owner(ui, egui::Id::new("editor_b")),
                "B не владеет"
            );
            keyboard_clear_owner(ui);
            assert!(!keyboard_is_owner(ui, id_a), "после clear владельца нет");
        });
    }

    #[test]
    fn owner_transfer_a_to_b_keeps_owner_b() {
        // Сценарий: фокус перемещается с A на B. Владельцем становится B,
        // а A при своём запоздалом `lost_focus` (egui задерживает сигнал)
        // не должен «затирать» владельца B.
        with_ui(|ui| {
            let _ = register_controller(ui.ctx());
            let id_a = egui::Id::new("editor_a");
            let id_b = egui::Id::new("editor_b");

            // Кадр N: A получает фокус → становится владельцем.
            keyboard_set_owner(ui, id_a);
            assert!(keyboard_is_owner(ui, id_a));

            // Кадр N+1: B получил фокус → становится владельцем.
            keyboard_set_owner(ui, id_b);

            // Кадр N+2: A стреляет `lost_focus` (задержанный сигнал egui).
            // Проверка: A больше не владелец → не должен прятать клавиатуру.
            assert!(
                !keyboard_is_owner(ui, id_a),
                "A уже не владелец — не должен прятать клавиатуру"
            );
            assert!(keyboard_is_owner(ui, id_b), "B остаётся владельцем");
        });
    }

    #[test]
    fn buffer_persists_across_frames() {
        // Введённый текст не должен сбрасываться между кадрами и между полями:
        // буфер живёт в `KeyboardController.text_buffers` по id поля.
        let ctx = egui::Context::default();
        let (dispatch, _rx) = Dispatcher::<()>::new();
        let fixed_id = egui::Id::new("te_buffer_test");

        // Регистрируем контроллер (буферы в нём).
        let kb = KeyboardController::new(Arc::new(|| {}), Arc::new(|| {}));
        ctx.data_mut(|d| d.insert_temp(keyboard_controller_id(), kb));

        let f = std::cell::RefCell::new(Some(|ui: &mut UiWrapper| {
            // Кадр 1: буфер инициализируется от value="abc".
            TextEdit::<()>::new("abc")
                .id(fixed_id)
                .render(ui, &dispatch);
            let b = ui
                .ctx()
                .data(|d| {
                    d.get_temp::<KeyboardController>(keyboard_controller_id())
                        .map(|kb| kb.text_buffer(fixed_id))
                })
                .expect("буфер должен быть создан");
            assert_eq!(&*b.read().unwrap(), "abc", "буфер инициализирован от value");

            // egui вставил бы новый символ — имитируем: буфер меняется прямо.
            *b.write().unwrap() = "abcdef".to_owned();

            // Кадр 2: value="zzz" НЕ должно перезаписать буфер (он уже есть).
            TextEdit::<()>::new("zzz")
                .id(fixed_id)
                .render(ui, &dispatch);
            let b = ui
                .ctx()
                .data(|d| {
                    d.get_temp::<KeyboardController>(keyboard_controller_id())
                        .map(|kb| kb.text_buffer(fixed_id))
                })
                .expect("буфер должен существовать");
            assert_eq!(
                &*b.read().unwrap(),
                "abcdef",
                "буфер не перезаписывается новым value между кадрами"
            );
        }));
        let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let f = f.borrow_mut().take().unwrap();
                f(&mut UiWrapper::new_unconstrained(ui));
            });
        });
    }

    #[test]
    fn remember_set_inside_on_changed_works_when_guard_dropped() {
        // Регрессия deadlock: `email.set()` внутри `on_changed` НЕ должен
        // самоблокироваться на std::sync::RwLock. Условие — read-guard от
        // `rem.get()` не должен жить через `render()` в одном полном выражении.
        //
        // Анти-паттерн, который ПРИВОДИТ к deadlock:
        //   TextEdit::new(rem.get().clone()).on_changed(move |v| rem.set(v)...)
        //       .render(...)  // <- временный guard живёт до end::render -> set -> write
        // Безопасный паттерн — разорвать guard до построения виджета.
        let ctx = egui::Context::default();
        let (dispatch, _rx) = Dispatcher::<()>::new();
        let fixed_id = egui::Id::new("te_guard_dropped_test");

        // Кадр 1: рендерим пустое поле.
        {
            let f = std::cell::RefCell::new(Some(|ui: &mut UiWrapper| {
                let rem = crate::remember(ui, "safe_mem", || String::new());
                let init = rem.get().clone(); // guard уничтожается здесь
                let rem = rem.clone();
                TextEdit::<()>::new(init)
                    .id(fixed_id)
                    .on_changed(move |v| rem.set(v.to_owned()))
                    .render(ui, &dispatch);
            }));
            let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let f = f.borrow_mut().take().unwrap();
                    f(&mut UiWrapper::new_unconstrained(ui));
                });
            });
        }

        // Фокусируем и шлём Event::Text, как IME commitText.
        ctx.memory_mut(|m| m.request_focus(fixed_id));
        let raw = egui::RawInput {
            focused: true,
            events: vec![egui::Event::Text("a".into())],
            ..Default::default()
        };
        {
            let f = std::cell::RefCell::new(Some(|ui: &mut UiWrapper| {
                ui.ctx().memory_mut(|m| m.request_focus(fixed_id));
                let rem = crate::remember(ui, "safe_mem", || String::new());
                let init = rem.get().clone();
                let rem = rem.clone();
                TextEdit::<()>::new(init)
                    .id(fixed_id)
                    .on_changed(move |v| rem.set(v.to_owned()))
                    .render(ui, &dispatch);
            }));
            let _ = ctx.run_ui(raw, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let f = f.borrow_mut().take().unwrap();
                    f(&mut UiWrapper::new_unconstrained(ui));
                });
            });
        }

        // Не задеадлокило — и значение записалось в remember.
        let mut stored = None;
        {
            let f = std::cell::RefCell::new(Some(|ui: &mut UiWrapper| {
                let rem = crate::remember(ui, "safe_mem", || String::new());
                stored = Some(rem.get().clone());
            }));
            let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let mut f = f.borrow_mut().take().unwrap();
                    f(&mut UiWrapper::new_unconstrained(ui));
                });
            });
        }
        assert_eq!(
            stored.as_deref(),
            Some("a"),
            "remember должен содержать введённый текст"
        );
    }

    #[test]
    fn input_text_is_written_to_buffer_and_emits_callback() {
        // Интеграция: введённый символ попадает в буфер поля и вызывает `on_changed`.
        // Моделирует реальный ввод с клавиатуры через `Event::Text`.
        let ctx = egui::Context::default();
        let (dispatch, _rx) = Dispatcher::<()>::new();
        let fixed_id = egui::Id::new("te_input_test");

        // Регистрируем контроллер (нужен для буфера поля).
        let kb = KeyboardController::new(Arc::new(|| {}), Arc::new(|| {}));
        ctx.data_mut(|d| d.insert_temp(keyboard_controller_id(), kb));

        let seen = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));

        // Кадр 1: рендерим пустое поле.
        {
            let seen_cb = Arc::clone(&seen);
            let f = std::cell::RefCell::new(Some(|ui: &mut UiWrapper| {
                TextEdit::<()>::new("")
                    .id(fixed_id)
                    .on_changed(move |v| seen_cb.lock().unwrap().push(v.to_owned()))
                    .render(ui, &dispatch);
            }));
            let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let f = f.borrow_mut().take().unwrap();
                    f(&mut UiWrapper::new_unconstrained(ui));
                });
            });
        }

        // Фокусируем поле.
        ctx.memory_mut(|m| m.request_focus(fixed_id));

        // Кадр 2: подаём Event::Text("a") — поле вставляет в буфер, on_changed срабатывает.
        let raw = egui::RawInput {
            focused: true,
            events: vec![egui::Event::Text("a".into())],
            ..Default::default()
        };
        {
            let seen_cb = Arc::clone(&seen);
            let f = std::cell::RefCell::new(Some(|ui: &mut UiWrapper| {
                // Удерживаем фокус на поле в кадре ввода.
                ui.ctx().memory_mut(|m| m.request_focus(fixed_id));
                TextEdit::<()>::new("")
                    .id(fixed_id)
                    .on_changed(move |v| seen_cb.lock().unwrap().push(v.to_owned()))
                    .render(ui, &dispatch);
            }));
            let _ = ctx.run_ui(raw, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let f = f.borrow_mut().take().unwrap();
                    f(&mut UiWrapper::new_unconstrained(ui));
                });
            });
        }

        // Буфер поля должен содержать "a" (введённое значение).
        let buf = ctx
            .data(|d| {
                d.get_temp::<KeyboardController>(keyboard_controller_id())
                    .map(|kb| kb.text_buffer(fixed_id))
            })
            .expect("буфер должен существовать");
        assert_eq!(
            &*buf.read().unwrap(),
            "a",
            "введённый символ должен быть в буфере поля (сейчас {:?})",
            &*buf.read().unwrap()
        );

        // on_changed должен был вызваться хотя бы раз.
        assert!(
            !seen.lock().unwrap().is_empty(),
            "on_changed должен сработать при вводе"
        );

        // Поле должно удерживать фокус (иначе egui не рисует курсор).
        let focused = ctx.memory(|m| m.has_focus(fixed_id));
        assert!(
            focused,
            "поле должно быть сфокусировано — egui рисует курсор"
        );
    }

    #[test]
    fn android_input_type_mapping_matches_editorinfo() {
        // Маппиг `KeyboardType` -> InputType из Android SDK (android.text.InputType).
        use KeyboardType::*;
        const TYPE_CLASS_TEXT: i32 = 0x1;
        const TYPE_CLASS_NUMBER: i32 = 0x2;
        const TYPE_CLASS_PHONE: i32 = 0x3;
        const TYPE_TEXT_VARIATION_EMAIL_ADDRESS: i32 = 0x20;
        const TYPE_TEXT_VARIATION_URI: i32 = 0x10;
        const TYPE_TEXT_VARIATION_PASSWORD: i32 = 0x80;

        assert_eq!(super::android_input_type(Text), TYPE_CLASS_TEXT);
        assert_eq!(
            super::android_input_type(Email),
            TYPE_CLASS_TEXT | TYPE_TEXT_VARIATION_EMAIL_ADDRESS
        );
        assert_eq!(super::android_input_type(Phone), TYPE_CLASS_PHONE);
        assert_eq!(super::android_input_type(Number), TYPE_CLASS_NUMBER);
        assert_eq!(
            super::android_input_type(Password),
            TYPE_CLASS_TEXT | TYPE_TEXT_VARIATION_PASSWORD
        );
        assert_eq!(
            super::android_input_type(Uri),
            TYPE_CLASS_TEXT | TYPE_TEXT_VARIATION_URI
        );
    }

    #[test]
    fn android_ime_options_mapping_matches_editorinfo() {
        // Маппиг `ImeAction` -> imeOptions (android.view.inputmethod.EditorInfo).
        use ImeAction::*;
        const FLAG_NO_EXTRACT_UI: i32 = 0x1000_0000;
        const ACTION_GO: i32 = 0x2;
        const ACTION_SEARCH: i32 = 0x3;
        const ACTION_NEXT: i32 = 0x5;
        const ACTION_DONE: i32 = 0x6;

        assert_eq!(
            super::android_ime_options(Done),
            ACTION_DONE | FLAG_NO_EXTRACT_UI
        );
        assert_eq!(
            super::android_ime_options(Next),
            ACTION_NEXT | FLAG_NO_EXTRACT_UI
        );
        assert_eq!(
            super::android_ime_options(Search),
            ACTION_SEARCH | FLAG_NO_EXTRACT_UI
        );
        assert_eq!(
            super::android_ime_options(Go),
            ACTION_GO | FLAG_NO_EXTRACT_UI
        );
    }

    #[test]
    fn utf16_len_counts_surrogate_pairs() {
        // UTF-16 code units: BMP-символ = 1 unit, эмодзи (суррогатная пара) = 2.
        assert_eq!(super::utf16_len("abc"), 3);
        assert_eq!(super::utf16_len("привет"), 6); // кириллица — BMP, 1 unit/символ
        assert_eq!(super::utf16_len("a😀b"), 4); // 'a'+эмодзи(2)+'b'
        assert_eq!(super::utf16_len(""), 0);
    }

    #[test]
    fn utf16_char_index_maps_char_to_utf16_offset() {
        // char_index (символы) -> позиция в UTF-16 code units.
        let text = "a😀b";
        // 'a'(1), '😀'(2), 'b'(1)
        assert_eq!(super::utf16_char_index(text, 0), 0);
        assert_eq!(super::utf16_char_index(text, 1), 1); // после 'a'
        assert_eq!(super::utf16_char_index(text, 2), 3); // после '😀' (1+2)
        assert_eq!(super::utf16_char_index(text, 3), 4); // конец (1+2+1)
                                                         // запрос за границы — берём все символы
        assert_eq!(super::utf16_char_index(text, 10), 4);
        assert_eq!(super::utf16_char_index("", 0), 0);
    }

    #[test]
    fn ime_field_registry_builds_ordered_list() {
        // Рендер двух полей должен заполнить упорядоченный реестр [A, B]
        // (это базис для IME_ACTION_NEXT; сам переход фокуса делает платформа
        // по `next_ime_field_after` — покрыто в runtime).
        let ctx = egui::Context::default();
        let (dispatch, _rx) = Dispatcher::<()>::new();
        let id_a = egui::Id::new("te_nx_a");
        let id_b = egui::Id::new("te_nx_b");

        // Реестр теперь хранится в KeyboardController.
        let kb = KeyboardController::new(Arc::new(|| {}), Arc::new(|| {}));
        ctx.data_mut(|d| d.insert_temp(keyboard_controller_id(), kb));

        let f = std::cell::RefCell::new(Some(|ui: &mut UiWrapper| {
            TextEdit::<()>::new("")
                .id(id_a)
                .on_changed(|_| {})
                .render(ui, &dispatch);
            TextEdit::<()>::new("")
                .id(id_b)
                .on_changed(|_| {})
                .render(ui, &dispatch);
        }));
        let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let mut f = f.borrow_mut().take().unwrap();
                f(&mut UiWrapper::new_unconstrained(ui));
            });
        });

        let list = ctx
            .data(|d| {
                d.get_temp::<KeyboardController>(keyboard_controller_id())
                    .map(|kb| kb.registry_slot().read().unwrap().clone())
            })
            .expect("контроллер должен существовать");
        assert_eq!(list, vec![id_a, id_b], "порядок полей = порядок отрисовки");

        // И `next_ime_field_after` по этому реестру даёт B от A.
        assert_eq!(
            egui_android_runtime::next_ime_field_after(&list, id_a),
            Some(id_b)
        );
    }

    #[test]
    fn ime_commit_two_frame_delivery_does_not_deadlock() {
        // Воспроизводит реальный путь IME: commitText доставляется на
        // СЛЕДУЮЩЕМ кадре (`ime_deliver`), поле в фокусе, публикуется editor_state.
        // Основной риск — deadlock на `remember.set`/`ctx` при вставке текста.
        //
        // Шаблон из примера (после фикса): guard от get() разорван в переменную.
        let ctx = egui::Context::default();
        let (dispatch, _rx) = Dispatcher::<()>::new();
        let fixed_id = egui::Id::new("te_two_frame_commit");

        // Регистрируем KeyboardController с привязанным слотом editor-state
        // (как платформа в run.rs) — включаем публикацию состояния.
        let kb = KeyboardController::new(Arc::new(|| {}), Arc::new(|| {}));
        ctx.data_mut(|d| d.insert_temp(keyboard_controller_id(), kb));

        let last_value = Arc::new(std::sync::Mutex::new(String::new()));
        let last_a = Arc::clone(&last_value);
        let last_b = Arc::clone(&last_value);
        let last_c = Arc::clone(&last_value);

        // Кадр 0: инициализация remember + рендер (фокус ещё нет).
        {
            let f = std::cell::RefCell::new(Some(|ui: &mut UiWrapper| {
                let rem = crate::remember(ui, "two_frame_mem", || String::new());
                let init = rem.get().clone();
                let rem = rem.clone();
                let last = Arc::clone(&last_a);
                TextEdit::<()>::new(init)
                    .id(fixed_id)
                    .on_changed(move |v| {
                        rem.set(v.to_owned());
                        *last.lock().unwrap() = v.to_owned();
                    })
                    .render(ui, &dispatch);
            }));
            let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let mut f = f.borrow_mut().take().unwrap();
                    f(&mut UiWrapper::new_unconstrained(ui));
                });
            });
        }

        // Фокусируем.
        ctx.memory_mut(|m| m.request_focus(fixed_id));
        // Кадр 1: фокус закреплён, рендер.
        {
            let f = std::cell::RefCell::new(Some(|ui: &mut UiWrapper| {
                ui.ctx().memory_mut(|m| m.request_focus(fixed_id));
                let rem = crate::remember(ui, "two_frame_mem", || String::new());
                let init = rem.get().clone();
                let rem = rem.clone();
                let last = Arc::clone(&last_b);
                TextEdit::<()>::new(init)
                    .id(fixed_id)
                    .on_changed(move |v| {
                        rem.set(v.to_owned());
                        *last.lock().unwrap() = v.to_owned();
                    })
                    .render(ui, &dispatch);
            }));
            let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let mut f = f.borrow_mut().take().unwrap();
                    f(&mut UiWrapper::new_unconstrained(ui));
                });
            });
        }

        // Кадр 2: IME `ImeEvent::Commit` вставлен (как from `ime_deliver`).
        let raw = egui::RawInput {
            focused: true,
            events: vec![egui::Event::Ime(egui::ImeEvent::Commit("a".into()))],
            ..Default::default()
        };
        {
            let f = std::cell::RefCell::new(Some(|ui: &mut UiWrapper| {
                ui.ctx().memory_mut(|m| m.request_focus(fixed_id));
                let rem = crate::remember(ui, "two_frame_mem", || String::new());
                let init = rem.get().clone();
                let rem = rem.clone();
                let last = Arc::clone(&last_c);
                TextEdit::<()>::new(init)
                    .id(fixed_id)
                    .on_changed(move |v| {
                        rem.set(v.to_owned());
                        *last.lock().unwrap() = v.to_owned();
                    })
                    .render(ui, &dispatch);
            }));
            let _ = ctx.run_ui(raw, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let mut f = f.borrow_mut().take().unwrap();
                    f(&mut UiWrapper::new_unconstrained(ui));
                });
            });
        }

        // Если не задедлокило — текст должен был дойти до буфера/колбэка.
        let buffer = ctx
            .data(|d| {
                d.get_temp::<KeyboardController>(keyboard_controller_id())
                    .map(|kb| kb.text_buffer(fixed_id))
            })
            .expect("буфер поля должен существовать");
        assert_eq!(
            &*buffer.read().unwrap(),
            "a",
            "текст Commit должен попасть в буфер"
        );
        assert_eq!(
            &*last_value.lock().unwrap(),
            "a",
            "on_changed должен сработать"
        );
    }

    /// Регистрация контроллера + счетчики show/hide И захват последних
    /// `set_options`. Нужно для интеграционных тестов действий IME-кнопок
    /// (проверка show/hide и смены imeOptions при смене фокуса/владельца).
    fn register_controller_full(
        ctx: &egui::Context,
    ) -> (
        Arc<AtomicUsize>,
        Arc<AtomicUsize>,
        Arc<Mutex<Option<(i32, i32)>>>,
    ) {
        let show_count = Arc::new(AtomicUsize::new(0));
        let hide_count = Arc::new(AtomicUsize::new(0));
        let last = Arc::new(Mutex::new(None));
        let sc = Arc::clone(&show_count);
        let hc = Arc::clone(&hide_count);
        let last_b = Arc::clone(&last);
        let kb = KeyboardController::with_options(
            Arc::new(move || {
                sc.fetch_add(1, Ordering::SeqCst);
            }),
            Arc::new(move || {
                hc.fetch_add(1, Ordering::SeqCst);
            }),
            Arc::new(move |input_type, ime_options| {
                *last_b.lock().unwrap() = Some((input_type, ime_options));
            }),
        );
        ctx.data_mut(|d| d.insert_temp(keyboard_controller_id(), kb));
        (show_count, hide_count, last)
    }

    /// Проверка: при смене владельца фокуса поле обновляет EditorInfo
    /// (inputType + imeOptions) через `KeyboardController.set_options`.
    ///
    /// Моделирует переход по IME_ACTION_NEXT: поле A (Next) теряет фокус,
    /// поле B (Done) получает его и становится владельцем → должен быть
    /// вызван `set_options` с imeOptions Done. Это ключевой кейс — иначе
    /// после Next на клавиатуре останется кнопка Next вместо Done.
    #[test]
    fn ime_set_options_updates_editor_info_on_owner_change() {
        const ACTION_DONE: i32 = 6; // EditorInfo.IME_ACTION_DONE
        const ACTION_NEXT: i32 = 5; // EditorInfo.IME_ACTION_NEXT

        let ctx = egui::Context::default();
        let (dispatch, _rx) = Dispatcher::<()>::new();
        let (last_options, _set_count) = register_controller_with_options(&ctx);

        // Рендер поля A с ImeAction::Next — A становится владельцем и вызывает
        // set_options(inputType, imeOptions=NEXT).
        // (В тесте нет реального тапа, поэтому владельцем A делается через
        //  `render`, где при is_focused && !is_owner вызывается set_options.)
        ctx.memory_mut(|m| m.request_focus(egui::Id::new("te_opt_a")));
        {
            let f = std::cell::RefCell::new(Some(|ui: &mut UiWrapper| {
                TextEdit::<()>::new("")
                    .id(egui::Id::new("te_opt_a"))
                    .ime_action(ImeAction::Next)
                    .on_changed(|_| {})
                    .render(ui, &dispatch);
            }));
            let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let mut f = f.borrow_mut().take().unwrap();
                    f(&mut UiWrapper::new_unconstrained(ui));
                });
            });
        }

        // Рендер поля B с ImeAction::Done — B перетирает владельца и вызывает
        // set_options(inputType, imeOptions=DONE).
        ctx.memory_mut(|m| m.request_focus(egui::Id::new("te_opt_b")));
        {
            let f = std::cell::RefCell::new(Some(|ui: &mut UiWrapper| {
                TextEdit::<()>::new("")
                    .id(egui::Id::new("te_opt_b"))
                    .ime_action(ImeAction::Done)
                    .on_changed(|_| {})
                    .render(ui, &dispatch);
                // A тоже рендерится (но потерял фокус) — не должен затирать B.
                TextEdit::<()>::new("")
                    .id(egui::Id::new("te_opt_a"))
                    .ime_action(ImeAction::Next)
                    .on_changed(|_| {})
                    .render(ui, &dispatch);
            }));
            let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let mut f = f.borrow_mut().take().unwrap();
                    f(&mut UiWrapper::new_unconstrained(ui));
                });
            });
        }

        // После перехода фокуса A→B последний set_options должен быть от B:
        // imeOptions = DONE (а не NEXT от A). Из-за FLAG_NO_EXTRACT_UI проверяем
        // битовое И с ACTION_DONE/ACTION_NEXT.
        let last = *last_options.lock().unwrap();
        assert!(last.is_some(), "set_options должен был быть вызван");
        let (_ty, ime_options) = last.unwrap();
        assert_eq!(
            ime_options & 0xF,
            ACTION_DONE,
            "после перехода на B(Done) imeOptions должен быть DONE, фактически: 0x{:x}",
            ime_options
        );
        assert_ne!(
            ime_options & 0xF,
            ACTION_NEXT,
            "постed перехода imeOptions не должен остаться NEXT"
        );
    }

    /// Интеграция: получение фокуса (+ следующее по IME_ACTION_NEXT) вызывает
    /// `show()` и `set_options` у нового поля; потеря фокуса вызывает `hide()`
    /// и сброс владельца.
    ///
    /// Моделирует действие кнопки Next: поле A (Next) теряет фокус, фокус
    /// переходит на B (Done), B становится владельцем → `show()` + `set_options(Done)`.
    #[test]
    fn ime_next_transfers_focus_show_hide_and_options() {
        let ctx = egui::Context::default();
        let (dispatch, _rx) = Dispatcher::<()>::new();
        let (show_count, hide_count, last_options) = register_controller_full(&ctx);
        let id_a = egui::Id::new("te_nxt_a");
        let id_b = egui::Id::new("te_nxt_b");
        // Заполняем реестр полей (как платформа при двух request_focus),
        // иначе Next не найдёт следующее поле.
        ctx.data(|d| {
            let kb = d
                .get_temp::<KeyboardController>(keyboard_controller_id())
                .expect("контроллер зарегистрирован");
            *kb.registry_slot().write().unwrap() = vec![id_a, id_b];
        });

        // Кадр 1: A в фокусе (Next) → становится владельцем, show + set_options.
        ctx.memory_mut(|m| m.request_focus(id_a));
        {
            let f = std::cell::RefCell::new(Some(|ui: &mut UiWrapper| {
                ui.ctx().memory_mut(|m| m.request_focus(id_a));
                TextEdit::<()>::new("")
                    .id(id_a)
                    .ime_action(ImeAction::Next)
                    .on_changed(|_| {})
                    .render(ui, &dispatch);
            }));
            let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let mut f = f.borrow_mut().take().unwrap();
                    f(&mut UiWrapper::new_unconstrained(ui));
                });
            });
        }
        // A стал владельцем и показал клавиатуру.
        assert!(
            ctx.data(|d| {
                d.get_temp::<KeyboardController>(keyboard_controller_id())
                    .map(|kb| *kb.owner_slot().read().unwrap() == Some(id_a))
                    .unwrap_or(false)
            }),
            "A должен быть владельцем"
        );
        assert!(
            show_count.load(Ordering::SeqCst) >= 1,
            "show должен быть вызван для A"
        );

        // Кадр 2: фокус уходит A и приходит B (как при Next-переходе).
        ctx.memory_mut(|m| m.request_focus(id_b));
        {
            let f = std::cell::RefCell::new(Some(|ui: &mut UiWrapper| {
                ui.ctx().memory_mut(|m| m.request_focus(id_b));
                // A рендерится и теряет фокус → hide (A был владельцем).
                TextEdit::<()>::new("")
                    .id(id_a)
                    .ime_action(ImeAction::Next)
                    .on_changed(|_| {})
                    .render(ui, &dispatch);
                // B получает фокус → становится владельцем, show + set_options(Done).
                TextEdit::<()>::new("")
                    .id(id_b)
                    .ime_action(ImeAction::Done)
                    .on_changed(|_| {})
                    .render(ui, &dispatch);
            }));
            let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let mut f = f.borrow_mut().take().unwrap();
                    f(&mut UiWrapper::new_unconstrained(ui));
                });
            });
        }

        // Владелец теперь B, hide (для A-lost_focus) и show (для B) сработали,
        // set_options для B имеет imeOptions Done.
        assert!(
            ctx.data(|d| {
                d.get_temp::<KeyboardController>(keyboard_controller_id())
                    .map(|kb| *kb.owner_slot().read().unwrap() == Some(id_b))
                    .unwrap_or(false)
            }),
            "B должен стать владельцем"
        );
        assert!(
            hide_count.load(Ordering::SeqCst) >= 1,
            "hide должен быть вызван при потере фокуса"
        );
        let last = *last_options.lock().unwrap();
        assert_eq!(
            last.map(|(_, io)| io & 0xF),
            Some(6), // ACTION_DONE
            "set_options для B должен быть Done, фактически: {:?}",
            last
        );
    }

    /// Регрессия: после того, как системный Back скрыл клавиатуру и сбросил
    /// владельца (поле осталось в фокусе), повторный рендер с фокусом снова
    /// вызывает `show()` и восстанавливает владельца.
    ///
    /// Моделирует: фокус на поле → owner=A → системный Back (owner=None,
    /// поле не потеряло фокус) → следующий кадр → show() снова.
    #[test]
    fn ime_reopen_after_back_resets_owner_and_shows() {
        let ctx = egui::Context::default();
        let (dispatch, _rx) = Dispatcher::<()>::new();
        let (show_count, _hide_count) = register_controller(&ctx);
        let id = egui::Id::new("te_reopen_back");

        // Кадр 1: поле в фокусе → show + owner.
        ctx.memory_mut(|m| m.request_focus(id));
        {
            let f = std::cell::RefCell::new(Some(|ui: &mut UiWrapper| {
                ui.ctx().memory_mut(|m| m.request_focus(id));
                TextEdit::<()>::new("")
                    .id(id)
                    .on_changed(|_| {})
                    .render(ui, &dispatch);
            }));
            let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let mut f = f.borrow_mut().take().unwrap();
                    f(&mut UiWrapper::new_unconstrained(ui));
                });
            });
        }
        // Owner установлен, клавиатура показана.
        assert!(
            ctx.data(|d| {
                d.get_temp::<KeyboardController>(keyboard_controller_id())
                    .map(|kb| *kb.owner_slot().read().unwrap() == Some(id))
                    .unwrap_or(false)
            }),
            "поле должно стать владельцем"
        );
        assert!(
            show_count.load(Ordering::SeqCst) >= 1,
            "первый фокус должен показать"
        );

        // Системный Back: сбрасываем owner (поле НЕ теряет фокус в egui).
        ctx.data(|d| {
            let kb = d
                .get_temp::<KeyboardController>(keyboard_controller_id())
                .expect("контроллер есть");
            *kb.owner_slot().write().unwrap() = None;
        });
        let before_after_back = show_count.load(Ordering::SeqCst);

        // Кадр 2: поле всё ещё в фокусе (focused=true), owner пуст → show снова.
        let raw = egui::RawInput {
            focused: true,
            events: Vec::new(),
            ..Default::default()
        };
        {
            let f = std::cell::RefCell::new(Some(|ui: &mut UiWrapper| {
                ui.ctx().memory_mut(|m| m.request_focus(id));
                TextEdit::<()>::new("")
                    .id(id)
                    .on_changed(|_| {})
                    .render(ui, &dispatch);
            }));
            let _ = ctx.run_ui(raw, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let mut f = f.borrow_mut().take().unwrap();
                    f(&mut UiWrapper::new_unconstrained(ui));
                });
            });
        }
        // Owner восстановлен, show вызван снова.
        assert!(
            ctx.data(|d| {
                d.get_temp::<KeyboardController>(keyboard_controller_id())
                    .map(|kb| *kb.owner_slot().read().unwrap() == Some(id))
                    .unwrap_or(false)
            }),
            "после Back повторный фокус должен восстановить owner"
        );
        assert!(
            show_count.load(Ordering::SeqCst) > before_after_back,
            "после Back повторный фокус должен снова вызвать show()"
        );
        let _ = dispatch;
    }

    /// Регрессия: во время активного IME-ввода `publish_editor_state` должен
    /// публиковать КАРЕТКУ В КОНЦЕ текста (`selection_start == selection_end ==
    /// text_len`), а не выделение от начала (`0..len`).
    ///
    /// Баг с устройства (лог вход «привет»): egui хранит active preedit как
    /// выделение `0..2`, из-за чего `selection_start` становится 0, Gboard
    /// считает, что курсор в начале, и слает setComposingRegion от начала,
    /// затирая уже набранный текст (в поле остаётся только «ет» вместо «привет»).
    #[test]
    fn publish_editor_state_puts_caret_at_end_for_ime() {
        use egui_android_runtime::{ImeEditorState, ImeEditorStateSlot};

        let ctx = egui::Context::default();
        let slot: ImeEditorStateSlot = Arc::new(Mutex::new(None));

        // Регистрируем KeyboardController с привязанным слотом editor-state.
        let mut kb = KeyboardController::new(Arc::new(|| {}), Arc::new(|| {}));
        kb.bind_editor_state(Arc::clone(&slot));
        ctx.data_mut(|d| d.insert_temp(keyboard_controller_id(), kb));

        // Публикуем состояние с непустым текстом (имитация активного предикта «пр»).
        let id = egui::Id::new("te_publish_caret");
        let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let mut uw = UiWrapper::new_unconstrained(ui);
                keyboard_publish_editor_state(&mut uw, id, "пр");
            });
        });

        let state: Option<ImeEditorState> = slot.lock().unwrap().clone();
        let st = state.expect("состояние должно быть опубликовано");
        assert_eq!(st.text, "пр", "текст опубликован");
        assert_eq!(st.text_len, 2, "UTF-16 длина = 2");
        assert_eq!(
            st.selection_start, 2,
            "selection_start должен быть в конце текста (2), а не 0"
        );
        assert_eq!(
            st.selection_end, 2,
            "selection_end должен быть в конце текста (2), а не выделение 0..2"
        );
    }

    /// РЕГРЕССИЯ egui-слоя (лог PID 32342): после `ImeEvent::Preedit` с
    /// `replace_range` (наш commitText «привет ») egui оставляет выделение
    /// `0..7`, а НЕ каретку `7..7`. Поэтому следующая вставка (новое слово «к»)
    /// заменяет всё поле, и текст начинает вводиться заново.
    ///
    /// Ожидание: после preedit каретка в конце, и следующий ввод ДОПИСЫВАЕТСЯ.
    #[test]
    fn preedit_replace_range_then_insert_appends_not_overwrites() {
        let ctx = egui::Context::default();
        let (dispatch, _rx) = Dispatcher::<()>::new();
        let dispatch2 = dispatch.clone();
        let id = egui::Id::new("te_preedit_replace_caret");
        let captured = Arc::new(Mutex::new(String::new()));

        // Кадр 1: Preedit с replace_range — заменяет «привет» на «привет ».
        let raw1 = egui::RawInput {
            events: vec![egui::Event::Ime(egui::ImeEvent::Preedit {
                text: "привет ".into(),
                active_range_chars: Some(0..7),
                replace_range: Some(0..6),
            })],
            ..Default::default()
        };
        {
            let cap = Arc::clone(&captured);
            let f = std::cell::RefCell::new(Some(move |ui: &mut UiWrapper| {
                ui.ctx().memory_mut(|m| m.request_focus(id));
                let rem = crate::remember(ui, "te_preedit_remember", || "привет".to_owned());
                let init = rem.get().clone();
                let rem = rem.clone();
                let cap = Arc::clone(&cap);
                TextEdit::<()>::new(init)
                    .id(id)
                    .on_changed(move |v| {
                        rem.set(v.to_owned());
                        *cap.lock().unwrap() = v.to_owned();
                    })
                    .render(ui, &dispatch);
            }));
            let _ = ctx.run_ui(raw1, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let mut f = f.borrow_mut().take().unwrap();
                    f(&mut UiWrapper::new_unconstrained(ui));
                });
            });
        }
        assert_eq!(
            &*captured.lock().unwrap(),
            "привет ",
            "после Preedit текст должен замениться"
        );

        // Кадр 2: вставляем новое слово «к» как обычный Text.
        let raw2 = egui::RawInput {
            events: vec![egui::Event::Text("к".into())],
            ..Default::default()
        };
        {
            let cap = Arc::clone(&captured);
            let f = std::cell::RefCell::new(Some(move |ui: &mut UiWrapper| {
                ui.ctx().memory_mut(|m| m.request_focus(id));
                let rem = crate::remember(ui, "te_preedit_remember", || "привет".to_owned());
                let init = rem.get().clone();
                let rem = rem.clone();
                let cap = Arc::clone(&cap);
                TextEdit::<()>::new(init)
                    .id(id)
                    .on_changed(move |v| {
                        rem.set(v.to_owned());
                        *cap.lock().unwrap() = v.to_owned();
                    })
                    .render(ui, &dispatch2);
            }));
            let _ = ctx.run_ui(raw2, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let mut f = f.borrow_mut().take().unwrap();
                    f(&mut UiWrapper::new_unconstrained(ui));
                });
            });
        }

        assert_eq!(
            &*captured.lock().unwrap(),
            "привет к",
            "после preedit+replace_range следующая вставка ДОЛЖНА ДОПИСЫВАТЬ, а не затирать"
        );
    }
}
