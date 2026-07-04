//! Конфигурация платформы.

/// Конфигурация платформы.
///
/// Содержит настройки, общие для всех платформ.
#[derive(Clone)]
pub struct PlatformConfig {
    /// Тег для логгера.
    pub log_tag: String,

    /// Целевой FPS (кадров в секунду).
    pub target_fps: u32,
}

impl Default for PlatformConfig {
    fn default() -> Self {
        Self {
            log_tag: "egui_app".to_owned(),
            target_fps: 60,
        }
    }
}
