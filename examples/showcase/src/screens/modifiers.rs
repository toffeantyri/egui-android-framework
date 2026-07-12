//! ModifiersScreen — демонстрация модификаторов padding, size, background, align, clickable.

use egui_android_framework::{
    runtime::Dispatcher,
    ui::{
        containers::Column,
        modifier::{Modifier, ModifierDsl},
        remember,
        theme::Theme,
        widgets::{Button, Spacer, Text, Widget},
        UiWrapper,
    },
};

use crate::root_component::RootMsg;

/// Экран демонстрации модификаторов.
pub struct ModifiersScreen;

impl ModifiersScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<RootMsg>) {
        let c = &Theme::current_from_ui(ui).colors;
        let click_count = remember(ui, "mod_click_count", || 0i32);

        Column::new()
            .scrollable()
            .show(ui, dispatch, |ui, dispatch| {
                Text::new("Модификаторы")
                    .modifier(Modifier::new().padding(8.0))
                    .render(ui, dispatch);
                Spacer::new(8.0).render(ui, dispatch);

                // Padding
                Text::new("Padding 8px:").render(ui, dispatch);
                Text::new("Текст с padding")
                    .text_color(c.on_secondary)
                    .modifier(Modifier::new().background(c.secondary).padding(8.0))
                    .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);

                // Background
                Text::new("Background + padding (secondary):").render(ui, dispatch);
                Text::new("Secondary фон")
                    .text_color(c.on_secondary)
                    .modifier(
                        Modifier::new()
                            .fill_max_width()
                            .background(c.secondary)
                            .padding(12.0),
                    )
                    .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);

                // Size + Background
                Text::new("Size (200x48) + Background:").render(ui, dispatch);
                Text::new("200x48")
                    .text_color(c.on_secondary)
                    .modifier(
                        Modifier::new()
                            .width(200.0)
                            .height(48.0)
                            .background(c.secondary),
                    )
                    .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);

                // Clickable через модификатор (диспатчит RootMsg::Back)
                Text::new("Clickable (нажми на текст):").render(ui, dispatch);
                Text::new("Нажми меня — назад")
                    .text_color(c.on_secondary)
                    .modifier(
                        Modifier::new()
                            .padding(8.0)
                            .background(c.secondary)
                            .clickable(RootMsg::Back),
                    )
                    .render(ui, dispatch);

                // Счётчик через remember внутри замыкания
                Spacer::new(8.0).render(ui, dispatch);
                Text::new("Счётчик (remember внутри замыкания):").render(ui, dispatch);
                Text::new(format!("Значение: {}", click_count.get()))
                    .text_color(c.on_secondary)
                    .modifier(Modifier::new().padding(8.0).background(c.secondary))
                    .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);
                Button::new("← Назад")
                    .on_click(RootMsg::Back)
                    .colors(c.primary, c.primary)
                    .text_color(c.on_primary)
                    .modifier(Modifier::new().fill_max_width().padding(8.0))
                    .render(ui, dispatch);
            });
    }
}
