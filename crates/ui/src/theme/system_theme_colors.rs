//! Цвета фона UI в зависимости от системной темы.
//!
//! Позволяет приложению задать цвета фона для светлой и тёмной темы,
//! а затем получать актуальный цвет в зависимости от `SystemTheme`.
//!
//! # Пример
//!
//! ```rust,ignore
//! // При старте приложения:
//! use egui_android_ui::theme::system_theme_colors::set_ui_theme_colors;
//! set_ui_theme_colors(Color32::WHITE, Color32::from_rgb(28, 27, 31));
//!
//! // В render:
//! let bg = ui_background_color(SystemTheme::Dark);
//! ```

use egui::Color32;
use egui_android_core::SystemTheme;
use std::sync::OnceLock;

/// Цвета фона UI для светлой и тёмной темы.
#[derive(Clone, Copy, Debug)]
pub struct UiThemeColors {
    /// Цвет фона для светлой темы.
    pub light: Color32,
    /// Цвет фона для тёмной темы.
    pub dark: Color32,
}

static UI_THEME_COLORS: OnceLock<UiThemeColors> = OnceLock::new();

/// Установить цвета фона UI для светлой и тёмной темы.
pub fn set_ui_theme_colors(light: Color32, dark: Color32) {
    let _ = UI_THEME_COLORS.set(UiThemeColors { light, dark });
}

/// Получить цвет фона UI в зависимости от темы.
///
/// Если цвета не были установлены через `set_ui_theme_colors` — возвращает
/// серый по умолчанию (`Color32::from_gray(40)`).
pub fn ui_background_color(theme: SystemTheme) -> Color32 {
    let colors = UI_THEME_COLORS.get().copied().unwrap_or(UiThemeColors {
        light: Color32::WHITE,
        dark: Color32::from_gray(28),
    });
    match theme {
        SystemTheme::Light => colors.light,
        SystemTheme::Dark => colors.dark,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // OnceLock не сбрасывается между тестами — тесты проверяют
    // только что API работает, без привязки к порядку выполнения.

    #[test]
    fn test_returns_color32() {
        // Просто проверяем, что функция возвращает Color32 (не паника)
        let _ = ui_background_color(SystemTheme::Light);
        let _ = ui_background_color(SystemTheme::Dark);
    }
}
