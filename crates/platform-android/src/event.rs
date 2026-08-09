//! Типы событий платформы.
//!
//! Содержит типы, которыми backend обменивается с Runtime.
//! Вынесены из `backend/mod.rs` для разделения ответственности.
//!
//! # Архитектура
//!
//! `BackendEvent` — единый enum для всех событий от backend к run.rs:
//! - Lifecycle (InitWindow, Resume, Pause, Stop, Destroy)
//! - Input (Touch, PointerButton, Key)
//! - TextInput (IME)
//! - InsetsChanged, DpiChanged
//!
//! Runtime (run.rs) обрабатывает эти события и конвертирует их в egui-события.
//! Backend'ы (GlBackend, NativeBackend) создают эти события.

#![cfg(target_os = "android")]

/// Ошибка backend'а.
#[derive(Debug)]
pub enum BackendError {
    /// EGL инициализация не удалась.
    EglInit(String),
    /// Нет NativeWindow.
    NoNativeWindow,
    /// Пересоздание EGL surface не удалось.
    RecreateSurface(String),
    /// Прочие ошибки.
    Other(String),
}

impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendError::EglInit(msg) => write!(f, "EGL init: {}", msg),
            BackendError::NoNativeWindow => write!(f, "нет NativeWindow"),
            BackendError::RecreateSurface(msg) => write!(f, "recreate surface: {}", msg),
            BackendError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

/// Отступы (WindowInsets) в точках.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Insets {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Default for Insets {
    fn default() -> Self {
        Self {
            left: 0.0,
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
        }
    }
}

/// Событие бэкенда.
///
/// Пробрасывается из backend в Runtime для конвертации в egui-события.
pub enum BackendEvent {
    /// Событие жизненного цикла.
    Lifecycle(LifecycleEvent),
    /// Событие ввода (touch, key).
    Input(InputEvent),
    /// Изменение WindowInsets.
    InsetsChanged(Insets),
    /// Изменение DPI.
    DpiChanged(f32),
}

/// Событие жизненного цикла.
#[derive(Debug, Clone, Copy)]
pub enum LifecycleEvent {
    /// Окно инициализировано (или пересоздано).
    InitWindow,
    /// Приложение возобновило работу.
    Resume,
    /// Приложение приостановлено.
    Pause,
    /// Приложение остановлено.
    Stop,
    /// Приложение уничтожено.
    Destroy,
}

/// Событие ввода.
pub enum InputEvent {
    /// Сенсорное событие.
    Touch { phase: TouchPhase, pos: egui::Pos2 },
    /// Событие кнопки указателя.
    PointerButton { pos: egui::Pos2, pressed: bool },
    /// Событие клавиши.
    Key { key_code: i32, action: KeyAction },
}

/// Фаза сенсорного события.
#[derive(Debug, Clone, Copy)]
pub enum TouchPhase {
    Start,
    Move,
    End,
    Cancel,
}

/// Действие клавиши.
#[derive(Debug, Clone, Copy)]
pub enum KeyAction {
    Down,
    Up,
}

/// IME-команда, пришедшая из Kotlin `InputConnection` через JNI.
///
/// Эти команды генерируются на главном Java-потоке (в `EguiImeView`
/// InputConnection) и доставляются в главный цикл через потокобезопасную
/// очередь в `PlatformState`. В цикле они конвертируются в `egui::Event`.
///
/// Только здесь получается контент для ввода — никакого использования
/// `InputEvent::TextEvent` / `setTextInputState` / `textInputState()`.
#[derive(Debug, Clone)]
pub enum ImeCmd {
    /// `commitText(text)` — финальный текст (как правило, одна строка/символ).
    /// Конвертируется в `egui::ImeEvent::Commit(text)` — финализирует IME-композицию.
    Commit(String),
    /// `setComposingText(text)` — промежуточный предредактируемый текст (composition).
    /// Используется для отображения preedit, не ломая буфер.
    Composing(String),
    /// `performEditorAction(Next)` — перейти к следующему TextEdit.
    Next,
    /// `performEditorAction(Done/Search/Go)` — завершить редактирование, скрыть клавиатуру.
    Done,
    /// `deleteSurroundingText(before, after)` — удалить текст вокруг курсора.
    DeleteSurrounding { before: i32, after: i32 },
    /// `setComposingText` с диапазоном (start..end) в текущей строке.
    ComposingRange { text: String, start: i32, end: i32 },
    /// `setSelection(start, end)` — пользователь переставил курсор/выделение.
    /// Позиции — в UTF-16 code units (как Android). В текущей версии полностью
    /// применить к egui-курсору сложно; инкапсулируется как no-op (см.
    /// `process_ime_cmd`).
    SetSelection { start: i32, end: i32 },
    /// `beginBatchEdit()` — начало пакетной операции IME. Пока буфер открыт,
    /// текстовые события буферизуются, а не пушатся в `ime_pending`.
    BeginBatchEdit,
    /// `endBatchEdit()` — завершение пакетной операции. Накопленные события
    /// переносятся в `ime_pending`.
    EndBatchEdit,
}
