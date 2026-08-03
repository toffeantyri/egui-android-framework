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

use std::sync::Arc;

/// Ключ хранения контроллера клавиатуры в `egui::Context::data()`.
const KEYBOARD_CONTROLLER_ID: &str = "egui_keyboard_controller";

/// Callback для показа клавиатуры.
pub type KeyboardShowCallback = Arc<dyn Fn() + Send + Sync>;
/// Callback для скрытия клавиатуры.
pub type KeyboardHideCallback = Arc<dyn Fn() + Send + Sync>;

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
}

impl KeyboardController {
    /// Создать контроллер из двух callbac'ов.
    pub fn new(show: KeyboardShowCallback, hide: KeyboardHideCallback) -> Self {
        Self { show, hide }
    }

    /// Показать клавиатуру.
    pub fn show(&self) {
        (self.show)();
    }

    /// Скрыть клавиатуру.
    pub fn hide(&self) {
        (self.hide)();
    }
}

/// Получить `Id` для хранения [`KeyboardController`] в `egui::Context::data()`.
pub fn keyboard_controller_id() -> egui::Id {
    egui::Id::new(KEYBOARD_CONTROLLER_ID)
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
}
