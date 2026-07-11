//! Глобальное состояние темы: системная тема + override от приложения.
//!
//! Позволяет:
//! - Определить системную тему один раз через JNI.
//! - Переопределить тему из приложения.
//! - Получить текущую тему (с учётом override).
//! - Получить цвет заливки framebuffer (glClearColor) для текущей темы.
//!
//! # Пример
//!
//! ```rust,ignore
//! // При старте (в run.rs):
//! crate::theme::init_system_theme(system);
//!
//! // В кадре (run.rs):
//! let (r, g, b) = crate::theme::current_clear_color();
//! gl.clear_color(r, g, b, 1.0);
//! ```

use egui_android_core::SystemTheme;
use std::sync::atomic::{AtomicI8, AtomicU32, Ordering};

// -1 = не инициализировано, 0 = Light, 1 = Dark
static SYSTEM_THEME: AtomicI8 = AtomicI8::new(-1);
// -1 = нет override, 0 = Light, 1 = Dark
static OVERRIDE_THEME: AtomicI8 = AtomicI8::new(-1);

fn theme_to_i8(theme: SystemTheme) -> i8 {
    match theme {
        SystemTheme::Light => 0,
        SystemTheme::Dark => 1,
    }
}

fn i8_to_theme(val: i8) -> Option<SystemTheme> {
    match val {
        0 => Some(SystemTheme::Light),
        1 => Some(SystemTheme::Dark),
        _ => None,
    }
}

/// Инициализировать системную тему (вызывается один раз при старте).
pub fn init_system_theme(system: SystemTheme) {
    SYSTEM_THEME.store(theme_to_i8(system), Ordering::Relaxed);
}

/// Установить override темы.
/// `None` — вернуться к системной теме.
pub fn set_theme_override(theme: Option<SystemTheme>) {
    let val = match theme {
        Some(t) => theme_to_i8(t),
        None => -1,
    };
    OVERRIDE_THEME.store(val, Ordering::Relaxed);
}

/// Получить текущую тему: override, если установлен, иначе системную.
pub fn current_theme() -> SystemTheme {
    // Сначала проверяем override
    if let Some(t) = i8_to_theme(OVERRIDE_THEME.load(Ordering::Relaxed)) {
        return t;
    }
    // Иначе системная
    i8_to_theme(SYSTEM_THEME.load(Ordering::Relaxed)).expect("init_system_theme не был вызван")
}

/// Получить только системную тему (без учёта override).
pub fn system_theme() -> SystemTheme {
    i8_to_theme(SYSTEM_THEME.load(Ordering::Relaxed)).expect("init_system_theme не был вызван")
}

// ─── Clear color для framebuffer ──────────────────────────────────────────────

/// Цвет заливки framebuffer под системными барами (glClearColor).
/// Хранится как упакованный 0xRRGGBB.
pub struct ClearColor {
    pub light: u32,
    pub dark: u32,
}

static CLEAR_COLOR: AtomicU32 = AtomicU32::new(0x33_33_33); // по умолч. серый

/// Установить clear color напрямую (упакованный 0xRRGGBB).
pub fn set_clear_color(packed: u32) {
    CLEAR_COLOR.store(packed, Ordering::Relaxed);
}

/// Получить текущий clear color как (r, g, b) в диапазоне 0.0..1.0.
pub fn current_clear_color() -> (f32, f32, f32) {
    let packed = CLEAR_COLOR.load(Ordering::Relaxed);
    let r = ((packed >> 16) & 0xFF) as f32 / 255.0;
    let g = ((packed >> 8) & 0xFF) as f32 / 255.0;
    let b = (packed & 0xFF) as f32 / 255.0;
    (r, g, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_and_system() {
        init_system_theme(SystemTheme::Light);
        assert_eq!(system_theme(), SystemTheme::Light);
        assert_eq!(current_theme(), SystemTheme::Light);
    }

    #[test]
    fn test_override_to_dark() {
        init_system_theme(SystemTheme::Light);
        set_theme_override(Some(SystemTheme::Dark));
        assert_eq!(current_theme(), SystemTheme::Dark);
        // Системная тема не изменилась
        assert_eq!(system_theme(), SystemTheme::Light);
    }

    #[test]
    fn test_override_to_light_on_dark_system() {
        init_system_theme(SystemTheme::Dark);
        set_theme_override(Some(SystemTheme::Light));
        assert_eq!(current_theme(), SystemTheme::Light);
    }

    #[test]
    fn test_override_none_returns_system() {
        init_system_theme(SystemTheme::Dark);
        set_theme_override(Some(SystemTheme::Light));
        assert_eq!(current_theme(), SystemTheme::Light);
        set_theme_override(None);
        assert_eq!(current_theme(), SystemTheme::Dark);
    }
}
