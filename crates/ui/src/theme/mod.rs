//! Основная структура темы — объединяет цвета, типографику и скругления.

mod colors;
mod colors_palette;
mod material;
mod shapes;
mod system_theme_colors;
mod typography;

pub use colors::ColorPalette;
pub use colors_palette::Colors;
pub use material::MaterialTheme;
pub use shapes::Shapes;
pub use system_theme_colors::{set_ui_theme_colors, ui_background_color, UiThemeColors};
pub use typography::{FontWeight, TextStyle, Typography};

use egui::Id;

fn theme_key() -> Id {
    Id::new("egui_android_theme")
}

#[derive(Clone, Debug)]
pub struct Theme {
    pub colors: ColorPalette,
    pub typography: Typography,
    pub shapes: Shapes,
}

impl Theme {
    pub fn apply(&self, ctx: &egui::Context) {
        ctx.data_mut(|data| {
            data.insert_temp(theme_key(), self.clone());
        });

        // Определяем, тёмная тема или светлая, по яркости фона
        let bg = self.colors.background;
        let is_dark = (bg.r() as u32 + bg.g() as u32 + bg.b() as u32) < 384;

        // Применяем стиль к обеим темам egui (Light и Dark), чтобы не зависеть
        // от того, какую тему использует CentralPanel.
        for &egui_theme in &[egui::Theme::Light, egui::Theme::Dark] {
            let mut style = ctx.style_of(egui_theme).as_ref().clone();

            style.visuals.dark_mode = is_dark;
            style.visuals.window_fill = self.colors.surface;
            style.visuals.panel_fill = self.colors.background;
            style.visuals.window_stroke = egui::Stroke::new(0.0, self.colors.outline_variant);
            // Фон поля ввода (TextEdit) — слой `surface_container_highest` (аналог
            // M3), чтобы поле выделялось на фоне, а не сливалось. `egui::TextEdit`
            // использует `visuals.extreme_bg_color` (или text_edit_bg_color).
            style.visuals.extreme_bg_color = self.colors.surface_container_highest;
            style.visuals.text_edit_bg_color = Some(self.colors.surface_container_highest);
            style.visuals.widgets.noninteractive.bg_fill = self.colors.surface;
            style.visuals.widgets.noninteractive.fg_stroke =
                egui::Stroke::new(1.0, self.colors.on_surface);
            style.visuals.widgets.inactive.bg_fill = self.colors.surface;
            style.visuals.widgets.inactive.fg_stroke =
                egui::Stroke::new(1.0, self.colors.on_surface);
            style.visuals.widgets.active.bg_fill = self.colors.primary;
            style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, self.colors.on_primary);
            style.visuals.widgets.hovered.bg_fill = self.colors.primary;
            style.visuals.widgets.hovered.fg_stroke =
                egui::Stroke::new(1.0, self.colors.on_primary);
            style.visuals.selection.bg_fill = self.colors.primary;
            style.visuals.selection.stroke = egui::Stroke::new(1.0, self.colors.on_primary);
            style.visuals.hyperlink_color = self.colors.primary;
            style.visuals.faint_bg_color = self.colors.surface;
            // Явно задаём цвет текста — иначе egui использует дефолтный серый
            style.visuals.override_text_color = Some(self.colors.on_background);

            ctx.set_style_of(egui_theme, style);
        }
    }

    pub fn current(ctx: &egui::Context) -> Self {
        ctx.data(|data| {
            data.get_temp::<Theme>(theme_key())
                .unwrap_or_else(MaterialTheme::light)
        })
    }

    pub fn current_from_ui(ui: &egui::Ui) -> Self {
        Self::current(ui.ctx())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Фон поля ввода (TextEdit) должен быть слоем `surface_container_highest`
    /// (аналог M3), чтобы выделяться на фоне, а не сливаться с ним.
    #[test]
    fn text_edit_background_matches_theme_surface() {
        let light = MaterialTheme::light();
        let dark = MaterialTheme::dark();

        // `Theme::apply` задаёт стиль обеим темам egui (Light и Dark)
        // с одним фоном поля. Проверяем обе.

        for &egui_theme in &[egui::Theme::Light, egui::Theme::Dark] {
            // Светлая тема
            let ctx = egui::Context::default();
            light.apply(&ctx);
            let bg = ctx.style_of(egui_theme).visuals.text_edit_bg_color();
            assert_eq!(
                bg, light.colors.surface_container_highest,
                "светлая: фон поля = surface_container_highest (egui-тема {egui_theme:?})"
            );
            assert_ne!(
                bg, light.colors.background,
                "светлая: фон поля должен отличаться от фона экрана (не сливаться)"
            );

            // Тёмная тема
            let ctx = egui::Context::default();
            dark.apply(&ctx);
            let bg = ctx.style_of(egui_theme).visuals.text_edit_bg_color();
            assert_eq!(
                bg, dark.colors.surface_container_highest,
                "тёмная: фон поля = surface_container_highest (egui-тема {egui_theme:?})"
            );
            assert_ne!(
                bg, dark.colors.background,
                "тёмная: фон поля должен отличаться от фона экрана (не сливаться)"
            );
            assert!(
                (bg.r() as u32 + bg.g() as u32 + bg.b() as u32) > 60,
                "тёмная: фон поля не должен быть почти чёрным (получилось {bg:?})"
            );
        }
    }
}
