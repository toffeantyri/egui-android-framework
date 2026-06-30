//! Тесты для системы тем (ColorPalette, Typography, Theme, MaterialTheme).
//!
//! Проверяет:
//! - Установка и чтение темы через egui::Context
//! - Тема по умолчанию — светлая
//! - Светлая и тёмная темы различаются
//! - ColorPalette, Typography, Shapes создаются
//! - Theme::current_from_ui работает

use crate::{
    theme::{ColorPalette, Shapes, TextStyle, Typography},
    FontWeight, MaterialTheme, Theme,
};

// ─── Theme ──────────────────────────────────────────────────────────────────

#[test]
fn theme_can_be_applied_and_retrieved() {
    let ctx = egui::Context::default();
    let theme = MaterialTheme::dark();
    theme.apply(&ctx);

    let retrieved = Theme::current(&ctx);
    assert_eq!(retrieved.colors.primary, theme.colors.primary);
    assert_eq!(retrieved.colors.background, theme.colors.background);
    assert_eq!(retrieved.shapes.medium, theme.shapes.medium);
}

#[test]
fn theme_defaults_to_light_if_not_set() {
    let ctx = egui::Context::default();
    let theme = Theme::current(&ctx);
    assert_eq!(theme.colors.primary, MaterialTheme::light().colors.primary);
    assert_eq!(
        theme.colors.background,
        MaterialTheme::light().colors.background
    );
}

#[test]
fn theme_can_be_overwritten() {
    let ctx = egui::Context::default();
    MaterialTheme::light().apply(&ctx);
    MaterialTheme::dark().apply(&ctx);

    let retrieved = Theme::current(&ctx);
    assert_eq!(
        retrieved.colors.background,
        MaterialTheme::dark().colors.background
    );
}

#[test]
fn theme_current_from_ui_works() {
    let ctx = egui::Context::default();
    MaterialTheme::dark().apply(&ctx);

    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let theme = Theme::current_from_ui(ui);
            assert_eq!(theme.colors.primary, MaterialTheme::dark().colors.primary);
        });
    });
}

#[test]
fn theme_is_cloneable() {
    let theme = MaterialTheme::light();
    let cloned = theme.clone();
    assert_eq!(theme.colors.primary, cloned.colors.primary);
    assert_eq!(theme.shapes.large, cloned.shapes.large);
}

// ─── MaterialTheme ──────────────────────────────────────────────────────────

#[test]
fn material_theme_light_and_dark_are_different() {
    let light = MaterialTheme::light();
    let dark = MaterialTheme::dark();
    assert_ne!(light.colors.background, dark.colors.background);
    assert_ne!(light.colors.primary, dark.colors.primary);
    assert_ne!(light.colors.surface, dark.colors.surface);
}

#[test]
fn material_theme_light_has_white_background() {
    let theme = MaterialTheme::light();
    assert_eq!(theme.colors.background, egui::Color32::WHITE);
}

#[test]
fn material_theme_dark_has_dark_background() {
    let theme = MaterialTheme::dark();
    assert_eq!(theme.colors.background, egui::Color32::from_rgb(18, 18, 18));
}

// ─── ColorPalette ───────────────────────────────────────────────────────────

#[test]
fn color_palette_can_be_created() {
    let palette = ColorPalette {
        primary: egui::Color32::RED,
        on_primary: egui::Color32::WHITE,
        secondary: egui::Color32::BLUE,
        on_secondary: egui::Color32::WHITE,
        background: egui::Color32::BLACK,
        on_background: egui::Color32::WHITE,
        surface: egui::Color32::from_gray(30),
        on_surface: egui::Color32::WHITE,
        error: egui::Color32::RED,
        on_error: egui::Color32::WHITE,
    };
    assert_eq!(palette.primary, egui::Color32::RED);
    assert_eq!(palette.on_primary, egui::Color32::WHITE);
}

// ─── Typography ─────────────────────────────────────────────────────────────

#[test]
fn typography_default_has_reasonable_values() {
    let typography = Typography::default();
    assert_eq!(typography.h1.font_size, 32.0);
    assert_eq!(typography.h2.font_size, 24.0);
    assert_eq!(typography.h3.font_size, 18.0);
    assert_eq!(typography.body.font_size, 14.0);
    assert_eq!(typography.caption.font_size, 12.0);
}

#[test]
fn text_style_can_be_created() {
    let style = TextStyle {
        font_size: 20.0,
        font_weight: FontWeight::Bold,
        color: egui::Color32::YELLOW,
    };
    assert_eq!(style.font_size, 20.0);
}

// ─── Shapes ─────────────────────────────────────────────────────────────────

#[test]
fn shapes_have_reasonable_values() {
    let shapes = Shapes {
        small: 4.0,
        medium: 8.0,
        large: 16.0,
    };
    assert_eq!(shapes.small, 4.0);
    assert_eq!(shapes.medium, 8.0);
    assert_eq!(shapes.large, 16.0);
}
