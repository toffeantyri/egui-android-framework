//! Состояние темы: системная тема + override от приложения.
//!
//! Все функции работают через [`PlatformState`], сохранённый в `egui::Context`.
//! Глобальные статики отсутствуют — состояние хранится в backend и синхронизируется
//! с egui::Context при InitWindow.
//!
//! # Пример использования из Application
//!
//! ```rust,ignore
//! fn frame(&mut self, egui_ctx: &egui::Context, raw_input: egui::RawInput) -> egui::FullOutput {
//!     // Установить clear color в соответствии с темой приложения
//!     use egui_android_framework::platform_android::theme::set_clear_color_from;
//!     set_clear_color_from(egui_ctx, theme.colors.background);
//! }
//! ```

#![cfg(target_os = "android")]

use crate::platform_state::PlatformState;

/// Установить clear color из Color32 (через PlatformState в egui::Context).
///
/// Вызывается из `Application::frame()` для синхронизации цвета фона
/// с темой приложения.
pub fn set_clear_color_from(ctx: &egui::Context, color: egui::Color32) {
    if let Some(state) = PlatformState::from_ctx(ctx) {
        state.set_clear_color_from(color);
    }
}

/// Установить clear color напрямую (упакованный 0xRRGGBB).
pub fn set_clear_color(ctx: &egui::Context, packed: u32) {
    if let Some(state) = PlatformState::from_ctx(ctx) {
        state.set_clear_color(packed);
    }
}

/// Получить текущий clear color (r, g, b) из PlatformState.
pub fn current_clear_color(ctx: &egui::Context) -> (f32, f32, f32) {
    if let Some(state) = PlatformState::from_ctx(ctx) {
        state.current_clear_color()
    } else {
        (0.2, 0.2, 0.2) // fallback серый
    }
}

/// Получить текущую тему (с учётом override).
pub fn current_theme(ctx: &egui::Context) -> Option<egui_android_platform::SystemTheme> {
    PlatformState::from_ctx(ctx).map(|s| s.current_theme())
}

/// Установить системную тему.
pub fn init_system_theme(ctx: &egui::Context, theme: egui_android_platform::SystemTheme) {
    if let Some(state) = PlatformState::from_ctx(ctx) {
        state.set_system_theme(theme);
    }
}

/// Установить override темы от приложения.
pub fn set_theme_override(ctx: &egui::Context, theme: Option<egui_android_platform::SystemTheme>) {
    if let Some(state) = PlatformState::from_ctx(ctx) {
        state.set_theme_override(theme);
    }
}
