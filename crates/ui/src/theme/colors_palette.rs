//! Кастомные цвета для приложения.
//!
//! Позволяет обращаться к цветам по имени: `Colors::LIGHT_GREEN`, `Colors::EMERALD`.
//! Можно добавлять новые цвета по мере необходимости.
//!
//! # Пример
//!
//! ```ignore
//! use egui_android_framework::ui::theme::Colors;
//!
//! Text::new("Зелёный текст")
//!     .modifier(Modifier::new().background(Colors::EMERALD))
//!     .render(ui, dispatch);
//! ```

use egui::Color32;

/// Набор часто используемых цветов для приложения.
pub struct Colors;

impl Colors {
    // ─── Зелёные ───────────────────────────────────────────────────────

    /// Светло-зелёный (салатовый) — #2ECC71
    pub const LIGHT_GREEN: Color32 = Color32::from_rgb(46, 204, 113);
    /// Изумрудный — #27AE60
    pub const EMERALD: Color32 = Color32::from_rgb(39, 174, 96);
    /// Зелёный лесной — #1ABC9C
    pub const FOREST_GREEN: Color32 = Color32::from_rgb(26, 188, 156);
    /// Тёмно-зелёный — #0E6655
    pub const DARK_GREEN: Color32 = Color32::from_rgb(14, 102, 85);

    // ─── Синие ─────────────────────────────────────────────────────────

    /// Синий — #3498DB
    pub const BLUE: Color32 = Color32::from_rgb(52, 152, 219);
    /// Тёмно-синий — #2C3E50
    pub const DARK_BLUE: Color32 = Color32::from_rgb(44, 62, 80);
    /// Голубой — #5DADE2
    pub const LIGHT_BLUE: Color32 = Color32::from_rgb(93, 173, 226);

    // ─── Красные / Оранжевые ──────────────────────────────────────────

    /// Красный — #E74C3C
    pub const RED: Color32 = Color32::from_rgb(231, 76, 60);
    /// Оранжевый — #E67E22
    pub const ORANGE: Color32 = Color32::from_rgb(230, 126, 34);
    /// Жёлтый — #F1C40F
    pub const YELLOW: Color32 = Color32::from_rgb(241, 196, 15);

    // ─── Серые / Нейтральные ──────────────────────────────────────────

    /// Тёмно-серый — #34495E
    pub const DARK_GRAY: Color32 = Color32::from_rgb(52, 73, 94);
    /// Серый — #7F8C8D
    pub const GRAY: Color32 = Color32::from_rgb(127, 140, 141);
    /// Светло-серый — #BDC3C7
    pub const LIGHT_GRAY: Color32 = Color32::from_rgb(189, 195, 199);
    /// Белый — #FFFFFF
    pub const WHITE: Color32 = Color32::WHITE;
    /// Чёрный — #000000
    pub const BLACK: Color32 = Color32::BLACK;

    // ─── Фиолетовые ───────────────────────────────────────────────────

    /// Фиолетовый — #9B59B6
    pub const PURPLE: Color32 = Color32::from_rgb(155, 89, 182);
    /// Тёмно-фиолетовый — #8E44AD
    pub const DARK_PURPLE: Color32 = Color32::from_rgb(142, 68, 173);
}
