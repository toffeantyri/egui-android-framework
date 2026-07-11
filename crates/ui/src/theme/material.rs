//! Material Design 3 темы (светлая и тёмная).

use crate::theme::{ColorPalette, Shapes, Theme, Typography};

pub struct MaterialTheme;

impl MaterialTheme {
    pub fn light() -> Theme {
        Theme {
            colors: ColorPalette {
                primary: egui::Color32::from_rgb(103, 80, 164),
                on_primary: egui::Color32::WHITE,
                secondary: egui::Color32::from_rgb(98, 91, 113),
                on_secondary: egui::Color32::WHITE,
                background: egui::Color32::from_rgb(255, 251, 254),
                on_background: egui::Color32::from_rgb(28, 27, 31),
                surface: egui::Color32::from_rgb(255, 251, 254),
                on_surface: egui::Color32::from_rgb(28, 27, 31),
                error: egui::Color32::from_rgb(179, 38, 30),
                on_error: egui::Color32::WHITE,
                outline: egui::Color32::from_rgb(121, 116, 126),
                outline_variant: egui::Color32::from_rgb(196, 199, 197),
            },
            typography: Typography::default(),
            shapes: Shapes::default(),
        }
    }

    pub fn dark() -> Theme {
        Theme {
            colors: ColorPalette {
                primary: egui::Color32::from_rgb(208, 188, 255),
                on_primary: egui::Color32::from_rgb(55, 30, 115),
                secondary: egui::Color32::from_rgb(204, 194, 220),
                on_secondary: egui::Color32::from_rgb(50, 44, 64),
                background: egui::Color32::from_rgb(28, 27, 31),
                on_background: egui::Color32::from_rgb(230, 225, 229),
                surface: egui::Color32::from_rgb(28, 27, 31),
                on_surface: egui::Color32::from_rgb(230, 225, 229),
                error: egui::Color32::from_rgb(242, 184, 181),
                on_error: egui::Color32::from_rgb(96, 20, 12),
                outline: egui::Color32::from_rgb(147, 143, 153),
                outline_variant: egui::Color32::from_rgb(68, 71, 70),
            },
            typography: Typography::default(),
            shapes: Shapes::default(),
        }
    }
}
