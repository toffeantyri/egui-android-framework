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
    /// При первом нажатии — переключает цвет и возвращает true (Back перехвачен).
    /// При втором — возвращает false (RootComponent делает pop на Home).
    pub fn handle_back(&mut self) -> bool {
        match self.bg {
            BgColor::Blue => {
                self.bg = BgColor::Green;
                true // Back перехвачен, экран остаётся
            }
            BgColor::Green => {
                // Уже переключили — второй Back уходит на Root
                false
            }
        }
    }

    pub fn render(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<RootMsg>) {
        let bg_color = match self.bg {
            BgColor::Blue => egui::Color32::from_rgb(30, 60, 140),
            BgColor::Green => egui::Color32::from_rgb(30, 100, 50),
        };

        Column::new()
            .scrollable()
            .show(ui, dispatch, |ui, dispatch| {
                Text::new("Кастомная обработка Back")
                    .modifier(Modifier::new().padding(8.0))
                    .render(ui, dispatch);
                Spacer::new(8.0).render(ui, dispatch);

                Text::new(
                    "Нажмите системную кнопку Back — цвет фона переключится.\n\
                     Ещё раз Back — возврат на Home.",
                )
                .modifier(Modifier::new().padding(8.0))
                .render(ui, dispatch);

                Spacer::new(16.0).render(ui, dispatch);

                // Отображаем цвет фона
                let current = match self.bg {
                    BgColor::Blue => "Синий",
                    BgColor::Green => "Зелёный",
                };
                Text::new(format!("Текущий фон: {}", current))
                    .modifier(Modifier::new().padding(12.0).background(bg_color))
                    .render(ui, dispatch);

                Spacer::new(16.0).render(ui, dispatch);

                Button::new("← Назад (на Home)")
                    .on_click(RootMsg::Back)
                    .modifier(Modifier::new().fill_max_width().padding(8.0))
                    .render(ui, dispatch);
            });
    }
}
