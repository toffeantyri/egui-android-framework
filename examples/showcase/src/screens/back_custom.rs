//! BackCustomScreen — экран, где Back меняет цвет фона вместо pop.
//!
//! Демонстрирует кастомную обработку Back: при нажатии переключается
//! цвет фона между синим и зелёным. Back не делает pop — RootComponent
//! возвращается на Home только при повторном Back (через back_fallback).

use egui_android_framework::{
    runtime::Dispatcher,
    ui::{
        containers::Column,
        modifier::{Modifier, ModifierDsl},
        theme::Theme,
        widgets::{Button, Spacer, Text, Widget},
        UiWrapper,
    },
};

use crate::root_component::RootMsg;

/// Состояние фона: два цвета для переключения.
#[derive(Clone, Debug, PartialEq)]
enum BgColor {
    Blue,
    Green,
}

/// Экран с кастомной обработкой Back.
pub struct BackCustomScreen {
    bg: BgColor,
}

impl BackCustomScreen {
    pub fn new() -> Self {
        Self { bg: BgColor::Blue }
    }

    /// Обработать BackPressed — переключает цвет фона.
    pub fn handle_back(&mut self) -> bool {
        match self.bg {
            BgColor::Blue => {
                self.bg = BgColor::Green;
                true
            }
            BgColor::Green => false,
        }
    }

    pub fn render(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<RootMsg>) {
        let c = &Theme::current_from_ui(ui).colors;
        Column::new()
            .scrollable()
            .show(ui, dispatch, |ui, dispatch| {
                Text::new("Кастомная обработка Back")
                    .text_color(c.on_secondary)
                    .modifier(
                        Modifier::new()
                            .fill_max_width()
                            .padding(8.0)
                            .background(c.secondary),
                    )
                    .render(ui, dispatch);
                Spacer::new(8.0).render(ui, dispatch);

                Text::new(
                    "Нажмите системную кнопку Back — цвет фона переключится.\n\
                     Ещё раз Back — возврат на Home.",
                )
                .modifier(Modifier::new().padding(8.0))
                .render(ui, dispatch);

                Spacer::new(16.0).render(ui, dispatch);

                // Цветной блок с переключающимся фоном (кастомные цвета экрана)
                let current = match self.bg {
                    BgColor::Blue => "Синий",
                    BgColor::Green => "Зелёный",
                };
                let bg_color = match self.bg {
                    BgColor::Blue => c.primary,
                    BgColor::Green => c.secondary,
                };
                let on_color = match self.bg {
                    BgColor::Blue => c.on_primary,
                    BgColor::Green => c.on_secondary,
                };

                Text::new(format!("Текущий фон: {}", current))
                    .text_color(on_color)
                    .modifier(
                        Modifier::new()
                            .fill_max_width()
                            .padding(12.0)
                            .background(bg_color),
                    )
                    .render(ui, dispatch);

                Spacer::new(16.0).render(ui, dispatch);

                Button::new("← Назад (на Home)")
                    .on_click(RootMsg::Back)
                    .colors(c.primary, c.primary)
                    .text_color(c.on_primary)
                    .modifier(Modifier::new().fill_max_width().padding(8.0))
                    .render(ui, dispatch);
            });
    }
}
