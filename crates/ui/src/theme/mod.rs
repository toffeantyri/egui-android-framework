//! Основная структура темы — объединяет цвета, типографику и скругления.

mod colors;
mod material;
mod shapes;
mod typography;

pub use colors::ColorPalette;
pub use material::MaterialTheme;
pub use shapes::Shapes;
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
