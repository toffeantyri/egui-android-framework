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
use egui_android_runtime::{keyboard_controller_id, Dispatcher, KeyboardController};

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
/// ```ignore
/// let local = remember(ui, "email_input", || String::new());
/// TextEdit::new(&local.get())
///     .on_changed({
///         let local = local.clone();
///         move |v| local.set(v.to_owned())
///     })
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
        // «пропадает»). Буфер живёт в `Context::data()` и должен ключеваться по
        // ТОМУ ЖЕ id, что использует само поле. egui выбирает id поля как
        // `ui.next_auto_id()` (не `ui.id()`), поэтому мы заранее берём `field_id`
        // и передаём его и в буфер, и в `egui::TextEdit` через `.id(field_id)`,
        // чтобы ключи совпадали и не коллизировали между полями на одном экране.
        let field_id = self.field_id.unwrap_or_else(|| ui.next_auto_id());
        let ctx = ui.ctx().clone();
        let storage_key = field_buffer_key(field_id);

        let buffer_arc: Arc<RwLock<String>> = ctx
            .data(|d| d.get_temp::<Arc<RwLock<String>>>(storage_key))
            .unwrap_or_else(|| {
                let buf = Arc::new(RwLock::new(self.value.clone()));
                ctx.data_mut(|d| {
                    d.insert_temp(storage_key, Arc::clone(&buf));
                });
                buf
            });

        let mut text_guard = buffer_arc.write().expect("TextEdit: буфер poisoned");

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

            if is_focused {
                // Я — фокусный редактируемый editor. Если ещё не я владею клавиатурой
                // (вызов idempotent по кадрам, `show_soft_input` не спамится).
                if !keyboard_is_owner(ui, field_id) {
                    keyboard_show(ui);
                    keyboard_set_owner(ui, field_id);
                }
            } else if response.lost_focus() {
                // Я потерял фокус. Прячем клавиатуру ТОЛЬКО если я был её владельцем.
                // Если фокус ушёл на другой TextEdit — он уже стал владельцем (или станет
                // в этот же кадр), и это условие не сработает.
                if keyboard_is_owner(ui, field_id) {
                    keyboard_hide(ui);
                    keyboard_clear_owner(ui);
                }
            }
        }

        // ─── Изменение текста → callback ───
        if response.changed() {
            // Читаем фактическое значение из буфера (egui писал прямо в него).
            let new_value: String = text_guard.clone();
            // Приоритет: сначала локальный on_changed, затем on_change_msg.
            if let Some(cb) = &self.on_changed {
                cb(&new_value);
            }
            if let Some(cb) = &self.on_changed_msg {
                dispatch.dispatch(cb(new_value));
            }
        }

        // ─── Submit (Done / потеря фокуса) ───
        if response.lost_focus() {
            if let Some(cb) = &self.on_submit {
                cb(&text_guard);
            }
        }
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

/// Ключ хранения буфера текста поля в `Context::data()` (по `ui.id()` поля).
fn field_buffer_key(id: egui::Id) -> egui::Id {
    egui::Id::new(("egui_textedit_buffer", id))
}

/// Ключ общего состояния "владелец клавиатуры" (Id фокусного TextEdit).
fn keyboard_owner_key() -> egui::Id {
    egui::Id::new("egui_keyboard_owner")
}

/// Тип общего состояния владельца клавиатуры.
/// `Arc<RwLock<Option<Id>>>` — Send + Sync + Clone, хранится в `Context::data()`.
fn keyboard_owner_storage() -> Arc<RwLock<Option<egui::Id>>> {
    Arc::new(RwLock::new(None))
}

/// Является ли `id` текущим владельцем клавиатуры.
fn keyboard_is_owner(ui: &UiWrapper, id: egui::Id) -> bool {
    ui.ctx().data(|d| {
        d.get_temp::<Arc<RwLock<Option<egui::Id>>>>(keyboard_owner_key())
            .map(|s| {
                // Если lock poisoned — считаем, что владелец не мы (безопасно).
                s.read().ok().map_or(false, |guard| *guard == Some(id))
            })
            .unwrap_or(false)
    })
}

/// Назначить `id` владельцем клавиатуры.
fn keyboard_set_owner(ui: &UiWrapper, id: egui::Id) {
    ui.ctx().data_mut(|d| {
        let storage = d
            .get_temp::<Arc<RwLock<Option<egui::Id>>>>(keyboard_owner_key())
            .unwrap_or_else(keyboard_owner_storage);
        if let Ok(mut guard) = storage.write() {
            *guard = Some(id);
        }
        d.insert_temp(keyboard_owner_key(), storage);
    });
}

/// Сбросить владельца клавиатуры.
fn keyboard_clear_owner(ui: &UiWrapper) {
    ui.ctx().data_mut(|d| {
        let storage = d
            .get_temp::<Arc<RwLock<Option<egui::Id>>>>(keyboard_owner_key())
            .unwrap_or_else(keyboard_owner_storage);
        if let Ok(mut guard) = storage.write() {
            *guard = None;
        }
        d.insert_temp(keyboard_owner_key(), storage);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_android_runtime::KeyboardController;
    use std::sync::atomic::{AtomicUsize, Ordering};

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
        let kb = KeyboardController::new(
            Arc::new(move || {
                sc.fetch_add(1, Ordering::SeqCst);
            }),
            Arc::new(move || {
                hc.fetch_add(1, Ordering::SeqCst);
            }),
        );
        ctx.data_mut(|d| {
            d.insert_temp(keyboard_controller_id(), kb);
        });
        (show_count, hide_count)
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
        // буфер живёт в `Context::data()` по stable id поля. Явный `id` задаёт
        // детерминированный ключ — буфер и поле используют один и тот же id.
        let ctx = egui::Context::default();
        let (dispatch, _rx) = Dispatcher::<()>::new();
        let fixed_id = egui::Id::new("te_buffer_test");

        let f = std::cell::RefCell::new(Some(|ui: &mut UiWrapper| {
            // Кадр 1: буфер инициализируется от value="abc".
            TextEdit::<()>::new("abc")
                .id(fixed_id)
                .render(ui, &dispatch);
            let b = ui
                .ctx()
                .data(|d| d.get_temp::<Arc<RwLock<String>>>(field_buffer_key(fixed_id)))
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
                .data(|d| d.get_temp::<Arc<RwLock<String>>>(field_buffer_key(fixed_id)))
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
    fn input_text_is_written_to_buffer_and_emits_callback() {
        // Интеграция: введённый символ попадает в буфер поля и вызывает `on_changed`.
        // Моделирует реальный ввод с клавиатуры через `Event::Text`.
        let ctx = egui::Context::default();
        let (dispatch, _rx) = Dispatcher::<()>::new();
        let fixed_id = egui::Id::new("te_input_test");

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
            .data(|d| d.get_temp::<Arc<RwLock<String>>>(field_buffer_key(fixed_id)))
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
}
