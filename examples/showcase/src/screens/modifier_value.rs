//! ModifierValueScreen — демонстрация новой Modifier value type системы.
//!
//! Показывает все возможности Modifier<M>: padding, fill_max_width, size,
//! border, rounding, alpha, clickable и их комбинации.

use egui::Color32;
use egui_android_framework::{
    runtime::Dispatcher,
    ui::{
        containers::Column,
        modifier::legacy::ModifierExt,
        modifier::{Modifier, ModifierApply},
        widgets::{Button, Spacer, Text, Widget},
    },
};

use crate::root_component::RootMsg;

/// Экран демонстрации новой Modifier системы.
pub struct ModifierValueScreen;

impl ModifierValueScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<RootMsg>) {
        Column::new()
            .scrollable()
            .show(ui, dispatch, |ui, dispatch| {
                Text::new("Новая Modifier Value Type")
                    .modifier(Modifier::new().padding(8.0))
                    .render(ui, dispatch);
                Spacer::new(8.0).render(ui, dispatch);

                // 1. Padding
                Text::new("Padding all 16").render(ui, dispatch);
                Text::new("Текст с padding 16")
                    .modifier(Modifier::new().padding(16.0).background(Color32::DARK_GRAY))
                    .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);

                // 2. Fill max width
                Text::new("Fill max width").render(ui, dispatch);
                Text::new("На всю ширину")
                    .modifier(
                        Modifier::new()
                            .fill_max_width()
                            .background(Color32::from_rgb(20, 40, 80)),
                    )
                    .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);

                // 3. Fixed size
                Text::new("Fixed 200x48").render(ui, dispatch);
                Text::new("200x48")
                    .modifier(
                        Modifier::new()
                            .width(200.0)
                            .height(48.0)
                            .background(Color32::DARK_GREEN),
                    )
                    .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);

                // 4. Border + rounding
                Text::new("Border + rounding").render(ui, dispatch);
                Text::new("Рамка 2px, скругление 8px")
                    .modifier(
                        Modifier::new()
                            .padding(12.0)
                            .border(2.0, Color32::WHITE)
                            .rounding(8.0),
                    )
                    .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);

                // 5. Clickable (исправленный — только текст кликабелен)
                Text::new("Clickable (нажми — назад)").render(ui, dispatch);
                Text::new("Нажми меня (только текст!)")
                    .modifier(
                        Modifier::new()
                            .padding(8.0)
                            .background(Color32::from_rgb(60, 40, 80))
                            .clickable(RootMsg::Back),
                    )
                    .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);

                // 6. Комбинация всего
                Text::new("Комбинация").render(ui, dispatch);
                Text::new("Всё вместе")
                    .modifier(
                        Modifier::new()
                            .fill_max_width()
                            .padding(16.0)
                            .background(Color32::from_rgb(40, 40, 80))
                            .rounding(12.0)
                            .border(1.0, Color32::GRAY)
                            .alpha(0.9),
                    )
                    .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);

                // 7. padding_hv
                Text::new("padding_hv (гор 16, вер 8)").render(ui, dispatch);
                Text::new("Горизонтальный и вертикальный padding")
                    .modifier(
                        Modifier::new()
                            .padding_hv(16.0, 8.0)
                            .background(Color32::from_rgb(80, 40, 20)),
                    )
                    .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);

                // 8. Alpha (прозрачность)
                Text::new("Alpha 0.5 (прозрачность)").render(ui, dispatch);
                Text::new("Полупрозрачный текст")
                    .modifier(
                        Modifier::new()
                            .padding(8.0)
                            .background(Color32::BLUE)
                            .alpha(0.5),
                    )
                    .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);

                // 9. Кнопка назад
                Button::new("← Назад")
                    .on_click(RootMsg::Back)
                    .padding(8.0)
                    .render(ui, dispatch);
            });
    }
}
