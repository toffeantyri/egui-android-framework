//! Основная структура темы — объединяет цвета, типографику и скругления.
//!
//! Тема хранится в `egui::Context::data()` через ключ `THEME_KEY`.
//! Устанавливается в `Application::frame()` перед рендерингом.
//! Виджеты читают тему через [`Theme::current_from_ui`].

mod colors;
mod material;
mod shapes;
mod typography;

pub use colors::ColorPalette;
pub use material::MaterialTheme;
pub use shapes::Shapes;
pub use typography::{FontWeight, TextStyle, Typography};

use egui::Id;

/// Ключ для хранения темы в `egui::Context::data_mut()`.
fn theme_key() -> Id {
    Id::new("egui_android_theme")
}

/// Основная структура темы приложения.
///
/// Объединяет [`ColorPalette`], [`Typography`] и [`Shapes`].
///
/// # Установка темы
///
/// ```ignore
/// theme.apply(ctx);
/// ```
///
/// # Чтение темы
///
/// ```ignore
/// let theme = Theme::current_from_ui(ui);
/// ui.label(egui::RichText::new("Текст").color(theme.colors.on_surface));
/// ```
#[derive(Clone, Debug)]
pub struct Theme {
    /// Палитра цветов.
    pub colors: ColorPalette,
    /// Стили текста.
    pub typography: Typography,
    /// Скругления углов.
    pub shapes: Shapes,
}

impl Theme {
    /// Установить тему в контекст egui.
    ///
    /// Вызывается в `Application::frame()` до рендеринга виджетов.
    pub fn apply(&self, ctx: &egui::Context) {
        ctx.data_mut(|data| {
            data.insert_temp(theme_key(), self.clone());
        });
    }

    /// Получить текущую тему из контекста egui.
    ///
    /// Если тема не была установлена — возвращает светлую тему
    /// из [`MaterialTheme::light()`].
    pub fn current(ctx: &egui::Context) -> Self {
        ctx.data(|data| {
            data.get_temp::<Theme>(theme_key())
                .unwrap_or_else(MaterialTheme::light)
        })
    }

    /// Получить текущую тему из Ui (удобная обёртка).
    pub fn current_from_ui(ui: &egui::Ui) -> Self {
        Self::current(ui.ctx())
    }
}
