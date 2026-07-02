//! Интеграционные тесты AppError.

use egui_android_runtime::AppError;

#[test]
fn test_app_error_display() {
    let err = AppError::GraphicInit("GL не инициализирован".to_string());
    assert_eq!(
        err.to_string(),
        "Ошибка инициализации графики: GL не инициализирован"
    );

    let err = AppError::SurfaceCreation("Не удалось создать surface".to_string());
    assert_eq!(
        err.to_string(),
        "Ошибка создания поверхности: Не удалось создать surface"
    );

    let err = AppError::Lifecycle("Activity destroyed".to_string());
    assert_eq!(
        err.to_string(),
        "Ошибка жизненного цикла: Activity destroyed"
    );

    let err = AppError::Egl("EGL_BAD_DISPLAY".to_string());
    assert_eq!(err.to_string(), "Ошибка EGL: EGL_BAD_DISPLAY");
}

#[test]
fn test_app_error_debug() {
    let err = AppError::GraphicInit("test".to_string());
    assert!(format!("{:?}", err).contains("GraphicInit"));
}

#[test]
fn test_app_error_is_std_error() {
    use std::error::Error;
    let err = AppError::Egl("test".to_string());
    assert!(err.source().is_none());
}
