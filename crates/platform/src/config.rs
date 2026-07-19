//! Конфигурация платформы.

/// Конфигурация платформы.
///
/// Содержит настройки, специфичные для платформенного уровня:
/// - режим vsync
///
/// Настройки приложения (log_tag, target_fps) находятся в `RuntimeConfig`
/// в крейте `egui-android-runtime`.
#[derive(Clone, Default)]
pub struct PlatformConfig {
    /// Вертикальная синхронизация (ожидание vsync перед swap buffers).
    pub vsync: bool,
}
