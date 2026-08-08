//! Контроллер клавиатуры (IME) — мост между `ui` и `platform-android`.
//!
//! # Архитектура
//!
//! UI-слой (`egui-android-ui`) не может зависеть от `platform-android`
//! (а `platform-android` не должен зависеть от `ui`). Чтобы виджет
//! `TextEdit` мог** управлять клавиатурой по событию фокуса, используется
//! посредник через `egui::Context::data()`:
//!
//! - **Тип `KeyboardController` определён здесь, в runtime** — потому что
//!   и `ui`, и `platform-android` уже зависят от runtime (ноль новых граней
//!   в графе зависимостей, изоляция крейтов не нарушается).
//! - **`platform-android`** регистрирует callback (show/hide) в
//!   `egui::Context::data()` по `Id("egui_keyboard_controller")` при
//!   инициализации.
//! - **Виджет `TextEdit`** читает контроллер из `Context::data()` при
//!   `gained_focus`/`lost_focus` и вызывает `show()`/`hide()`.
//!
//! Если контроллер не зарегистрирован (десктоп, тесты, NativeBackend без IME)
//! — виджет просто пропускает управление клавиатурой без паники.
//!
//! Это подчиняется event-driven (push) архитектуре: клавиатура управляется
//! по событию фокуса, никакого polling нет.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};

/// Ключ хранения контроллера клавиатуры в `egui::Context::data()`.
const KEYBOARD_CONTROLLER_ID: &str = "egui_keyboard_controller";

/// Callback для показа клавиатуры.
pub type KeyboardShowCallback = Arc<dyn Fn() + Send + Sync>;
/// Callback для скрытия клавиатуры.
pub type KeyboardHideCallback = Arc<dyn Fn() + Send + Sync>;
/// Callback для обновления настроек клавиатуры (inputType + imeOptions, EditorInfo).
pub type KeyboardOptionsCallback = Arc<dyn Fn(i32, i32) + Send + Sync>;

/// Состояние редактирования активного (`focused`) `TextEdit`, публикуемое
/// ui-слой в платформу для двустороннего `InputConnection`.
///
/// Позиции заданы в **UTF-16 code units** (как Android `InputConnection`), т.к.
/// JNI-функции Kotlin работают с индексами `int` UTF-16. `Option` означает
/// отсутствие активной IME-композиции (preedit).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImeEditorState {
    /// Сфокусировано ли поле (есть ли активный редактор).
    pub focused: bool,
    /// Текущий текст поля (клонированный).
    pub text: String,
    /// Длина текста в UTF-16 code units.
    pub text_len: usize,
    /// Начало выделения в UTF-16 code units.
    pub selection_start: usize,
    /// Конец выделения в UTF-16 code units.
    pub selection_end: usize,
    /// Начало активной IME-композиции, если есть preedit.
    pub composing_start: Option<usize>,
    /// Конец активной IME-композиции, если есть preedit.
    pub composing_end: Option<usize>,
}

/// Общее хранилище состояния редактирования: ui-слой пишет, платформа (JNI)
/// читает. Опционально — заполняется фреймворком, а не пользователем.
pub type ImeEditorStateSlot = Arc<Mutex<Option<ImeEditorState>>>;

/// Контроллер клавиатуры (IME).
///
/// Регистрируется платформой (platform-android) при инициализации
/// в `egui::Context::data()` по [`keyboard_controller_id`].
///
/// Виджет `TextEdit` получает его через `ui.ctx().data()` по тому же Id
/// и вызывает `show()`/`hide()` при изменении фокуса.
#[derive(Clone)]
pub struct KeyboardController {
    /// Показать клавиатуру.
    show: KeyboardShowCallback,
    /// Скрыть клавиатуру.
    hide: KeyboardHideCallback,
    /// Обновить inputType + imeOptions (EditorInfo) через JNI.
    options: KeyboardOptionsCallback,
    /// Общее состояние редактора (ui -> platform) для двустороннего InputConnection.
    editor_state: Option<ImeEditorStateSlot>,
    /// Флаг «прейти к следующему полю» (Next), выставляется платформой,
    /// считывается ui-слой (`TextEdit`) для перевода фокуса.
    move_focus_next: Arc<AtomicBool>,
    /// Владелец клавиатуры (Id фокусного поля). Хранится тут (а не в
    /// `egui::Context::data`), чтобы ui-слой мог читать/писать его БЕЗ
    /// `ctx.data_mut` внутри render (reentrant write-lock на Context -> dead).
    owner_slot: Arc<RwLock<Option<egui::Id>>>,
    /// Упорядоченный реестр полей ввода (порядок отрисовки). Тоже вне
    /// `Context::data`, чтобы избежать `data_mut` в render.
    registry_slot: Arc<RwLock<Vec<egui::Id>>>,
    /// Буферы текста каждого поля — `Arc<RwLock<String>>` по `egui::Id`.
    /// Хранятся тут (а не в `Context::data`) — инициализация без `data_mut` внутри render.
    text_buffers: Arc<Mutex<std::collections::HashMap<egui::Id, Arc<RwLock<String>>>>>,
}

impl KeyboardController {
    /// Создать контроллер из show/hide callbac'ов (без настройки EditorInfo).
    ///
    /// `set_options` будет no-op. Для полной поддержки используйте
    /// [`Self::with_options`].
    pub fn new(show: KeyboardShowCallback, hide: KeyboardHideCallback) -> Self {
        let noop = Arc::new(|_input_type: i32, _ime_options: i32| {}) as KeyboardOptionsCallback;
        Self {
            show,
            hide,
            options: noop,
            editor_state: None,
            move_focus_next: Arc::new(AtomicBool::new(false)),
            owner_slot: Arc::new(RwLock::new(None)),
            registry_slot: Arc::new(RwLock::new(Vec::new())),
            text_buffers: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    /// Создать контроллер с поддержкой обновления EditorInfo (inputType/imeOptions).
    pub fn with_options(
        show: KeyboardShowCallback,
        hide: KeyboardHideCallback,
        options: KeyboardOptionsCallback,
    ) -> Self {
        Self {
            show,
            hide,
            options,
            editor_state: None,
            move_focus_next: Arc::new(AtomicBool::new(false)),
            owner_slot: Arc::new(RwLock::new(None)),
            registry_slot: Arc::new(RwLock::new(Vec::new())),
            text_buffers: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    /// Показать клавиатуру.
    pub fn show(&self) {
        (self.show)();
    }

    /// Скрыть клавиатуру.
    pub fn hide(&self) {
        (self.hide)();
    }

    /// Обновить `inputType` + `imeOptions` для IME (EditorInfo).
    ///
    /// Вызывается `TextEdit` при `gained_focus` с маппингом
    /// `KeyboardType`/`ImeAction` -> битовые маски Android. Если контроллер
    /// создан через `new()` (без options) — no-op.
    pub fn set_options(&self, input_type: i32, ime_options: i32) {
        (self.options)(input_type, ime_options);
    }

    /// Зарегистрировать общее хранилище состояния редактора для двустороннего
    /// `InputConnection`. Платформа создаёт `Arc<Mutex<Option<ImeEditorState>>>`,
    /// ui-слой (`TextEdit`) каждый кадр пишет туда текущее состояние фокусного
    /// поля, а JNI-функции читают его для `getTextBeforeCursor` и т.п.
    pub fn bind_editor_state(&mut self, slot: ImeEditorStateSlot) {
        self.editor_state = Some(slot);
    }

    /// Получить слот состояния редактора (если привязан платформой).
    pub fn editor_state(&self) -> Option<&ImeEditorStateSlot> {
        self.editor_state.as_ref()
    }

    /// Слот владельца клавиатуры (Id фокусного поля).
    pub fn owner_slot(&self) -> &Arc<RwLock<Option<egui::Id>>> {
        &self.owner_slot
    }

    /// Слот упорядоченного реестра полей ввода.
    pub fn registry_slot(&self) -> &Arc<RwLock<Vec<egui::Id>>> {
        &self.registry_slot
    }

    /// Получить или создать буфер текста для поля `id`.
    ///
    /// Возвращает `Arc<RwLock<String>>` — разделяемое, мутабельное хранилище
    /// текста поля, не требующее `ctx.data_mut` внутри render.
    pub fn text_buffer(&self, id: egui::Id) -> Arc<RwLock<String>> {
        let mut map = self.text_buffers.lock().unwrap();
        map.entry(id)
            .or_insert_with(|| Arc::new(RwLock::new(String::new())))
            .clone()
    }

    /// Попросить ui-слой перейти к следующему полю (`IME_ACTION_NEXT`).
    /// Вызывается платформой (loop.rs) при `ImeOutcome::Next`.
    pub fn request_move_focus_next(&self) {
        self.move_focus_next.store(true, Ordering::SeqCst);
    }

    /// Считать и сбросить флаг «перейти к следующему полю».
    /// Вызывается ui-слой (`TextEdit`) в кадре обработки.
    pub fn take_move_focus_next(&self) -> bool {
        self.move_focus_next.swap(false, Ordering::SeqCst)
    }
}

/// Получить `Id` для хранения [`KeyboardController`] в `egui::Context::data()`.
pub fn keyboard_controller_id() -> egui::Id {
    egui::Id::new(KEYBOARD_CONTROLLER_ID)
}

/// Найти следующее поле ввода после `current` в упорядоченном реестре полей
/// (порядок отрисовки).
///
/// Используется для `IME_ACTION_NEXT`: переход к следующему `TextEdit`.
/// Порядок задаёт упорядоченный список id полей (`ordered`). Если `current` не
/// найден в списке или `current` — последнее поле — возвращается `None`
/// (фокус не переводим; обычно IME закрывает клавиатуру).
pub fn next_ime_field_after(ordered: &[egui::Id], current: egui::Id) -> Option<egui::Id> {
    let pos = ordered.iter().position(|&id| id == current)?;
    ordered.get(pos + 1).copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_show_calls_underlying_callback() {
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_clone = Arc::clone(&calls);
        let kb = KeyboardController::new(
            Arc::new(move || {
                calls_clone.fetch_add(1, Ordering::SeqCst);
            }),
            Arc::new(|| {}),
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        kb.show();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_hide_calls_underlying_callback() {
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_clone = Arc::clone(&calls);
        let kb = KeyboardController::new(
            Arc::new(|| {}),
            Arc::new(move || {
                calls_clone.fetch_add(1, Ordering::SeqCst);
            }),
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        kb.hide();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_clone_constructs_controller() {
        // Контроллер можно создать и он клонируется (used in Context::data).
        let kb = KeyboardController::new(Arc::new(|| {}), Arc::new(|| {}));
        let _kb2 = kb.clone();
    }

    #[test]
    fn test_keyboard_controller_id_stable() {
        // Id должен быть стабильным (детерминированным).
        assert_eq!(
            keyboard_controller_id(),
            keyboard_controller_id(),
            "Id должен быть стабильным"
        );
    }

    #[test]
    fn test_set_options_calls_underlying_callback() {
        let calls = Arc::new(std::sync::Mutex::new(Vec::<(i32, i32)>::new()));
        let calls_clone = Arc::clone(&calls);
        let kb = KeyboardController::with_options(
            Arc::new(|| {}),
            Arc::new(|| {}),
            Arc::new(move |input_type, ime_options| {
                calls_clone.lock().unwrap().push((input_type, ime_options));
            }),
        );
        kb.set_options(1, 2);
        kb.set_options(0x8000, 0x05);
        assert_eq!(
            *calls.lock().unwrap(),
            vec![(1, 2), (0x8000, 0x05)],
            "set_options должен передать пары (inputType, imeOptions) платформе"
        );
    }

    #[test]
    fn test_set_options_noop_without_with_options() {
        // Контроллер, созданный через new(), должен молча игнорировать set_options.
        let kb = KeyboardController::new(Arc::new(|| {}), Arc::new(|| {}));
        kb.set_options(0x0b, 0x04); // не должно паниковать
    }

    #[test]
    fn test_bind_editor_state_and_publish() {
        // Двусторонний канал: bind_editor_state привязывает слот, ui-слой пишет,
        // платформа читает.
        let mut kb = KeyboardController::new(Arc::new(|| {}), Arc::new(|| {}));
        let slot: ImeEditorStateSlot = Arc::new(Mutex::new(None));
        kb.bind_editor_state(Arc::clone(&slot));

        // ui-слой публикует состояние.
        *slot.lock().unwrap() = Some(ImeEditorState {
            focused: true,
            text: "привет".to_owned(),
            text_len: "привет".encode_utf16().count(),
            selection_start: 3,
            selection_end: 3,
            composing_start: None,
            composing_end: None,
        });

        // Платформа читает через слот, полученный из контроллера.
        let read = kb.editor_state().unwrap().lock().unwrap().clone();
        let state = read.expect("состояние опубликовано");
        assert!(state.focused);
        assert_eq!(state.text, "привет");
        assert_eq!(state.text_len, 6); // 'п','р','и','в','е','т' = 6 UTF-16
        assert_eq!(state.selection_start, 3);
    }

    #[test]
    fn next_ime_field_after_returns_following_field() {
        let a = egui::Id::new("a");
        let b = egui::Id::new("b");
        let c = egui::Id::new("c");
        let ordered = [a, b, c];

        assert_eq!(next_ime_field_after(&ordered, a), Some(b));
        assert_eq!(next_ime_field_after(&ordered, b), Some(c));
        // Последнее поле — следующего нет (фокус не переводим).
        assert_eq!(next_ime_field_after(&ordered, c), None);
    }

    #[test]
    fn next_ime_field_after_handles_missing_and_empty() {
        let a = egui::Id::new("a");
        let b = egui::Id::new("b");
        let ordered = [a, b];

        // current не в реестре — None.
        assert_eq!(next_ime_field_after(&ordered, egui::Id::new("zzz")), None);
        // Пустой реестр — None.
        assert_eq!(next_ime_field_after(&[], a), None);
        // Один элемент — после него None.
        assert_eq!(next_ime_field_after(&[a], a), None);
    }

    #[test]
    fn move_focus_next_flag_sets_and_clears() {
        let kb = KeyboardController::new(Arc::new(|| {}), Arc::new(|| {}));
        // По умолчанию флаг сброшен.
        assert!(!kb.take_move_focus_next());
        // Платформа просит переход.
        kb.request_move_focus_next();
        assert!(kb.take_move_focus_next(), "флаг должен установиться");
        // Повторный take — снова false.
        assert!(!kb.take_move_focus_next(), "take сбрасывает флаг");
    }
}
