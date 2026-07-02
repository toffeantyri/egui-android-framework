//! Трейт [`Platform`] — контракт платформы.
//!
//! Каждая платформа (Android, iOS, Desktop, Web) реализует этот трейт.

use crate::config::PlatformConfig;

/// Абстракция платформы.
///
/// Параметры:
/// - `W` — тип окна (например, `NativeWindow` для Android)
/// - `I` — тип события ввода (например, `InputEvent` из android-activity)
///
/// Платформа отвечает за:
/// - lifecycle (Resume, Pause, Stop, Destroy)
/// - окно (InitWindow, изменение размеров)
/// - ввод (касания, клавиатура)
/// - цикл событий
///
/// Платформа НЕ знает про:
/// - бизнес-логику
/// - State, Reducer, Components
/// - конкретную реализацию рендеринга
pub trait Platform {
    /// Тип окна платформы.
    type Window: Clone + Send + 'static;

    /// Тип события ввода платформы.
    type InputEvent: Send + 'static;

    /// Тип ошибки платформы.
    type Error: std::fmt::Debug + Send + 'static;

    /// Запустить главный цикл приложения.
    ///
    /// Принимает приложение (generic `A`) и конфигурацию.
    /// Возвращает `Ok(())` при успешном завершении или ошибку.
    fn run<A>(app: A, config: PlatformConfig) -> Result<(), Self::Error>;
}
