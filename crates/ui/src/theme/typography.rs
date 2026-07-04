//! Типографика — шрифты, размеры, насыщенность.

#[derive(Clone, Copy, Debug)]
pub enum FontWeight {
    Light,
    Regular,
    Medium,
    Bold,
}

#[derive(Clone, Copy, Debug)]
pub enum TextStyle {
    DisplayLarge,
    DisplayMedium,
    DisplaySmall,
    HeadlineLarge,
    HeadlineMedium,
    HeadlineSmall,
    TitleLarge,
    TitleMedium,
    TitleSmall,
    BodyLarge,
    BodyMedium,
    BodySmall,
    LabelLarge,
    LabelMedium,
    LabelSmall,
}

#[derive(Clone, Debug)]
pub struct Typography {
    pub display_large: egui::FontId,
    pub display_medium: egui::FontId,
    pub display_small: egui::FontId,
    pub headline_large: egui::FontId,
    pub headline_medium: egui::FontId,
    pub headline_small: egui::FontId,
    pub title_large: egui::FontId,
    pub title_medium: egui::FontId,
    pub title_small: egui::FontId,
    pub body_large: egui::FontId,
    pub body_medium: egui::FontId,
    pub body_small: egui::FontId,
    pub label_large: egui::FontId,
    pub label_medium: egui::FontId,
    pub label_small: egui::FontId,
}

impl Default for Typography {
    fn default() -> Self {
        let props = egui::FontId::proportional;
        Self {
            display_large: props(32.0),
            display_medium: props(28.0),
            display_small: props(24.0),
            headline_large: props(22.0),
            headline_medium: props(20.0),
            headline_small: props(18.0),
            title_large: props(16.0),
            title_medium: props(14.0),
            title_small: props(13.0),
            body_large: props(16.0),
            body_medium: props(14.0),
            body_small: props(12.0),
            label_large: props(14.0),
            label_medium: props(12.0),
            label_small: props(11.0),
        }
    }
}
