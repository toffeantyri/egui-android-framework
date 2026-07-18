//! Вложенный экран внутри NestedScreen.
//!
//! Использует отдельный enum NestedRoute — не зависит от корневого Route.

use egui_android_core::{Component, LifecycleObserver, UiWrapper};
use egui_android_runtime::Dispatcher;
use egui_android_ui::{
    containers::Column,
    modifier::{Modifier, ModifierDsl},
    theme::Theme,
    widgets::{Button, Spacer, Text, Widget},
};

use crate::navigation::NestedRoute;
use crate::navigation_host::RootMsg;

/// Вложенный экран (Nested A, B, C).
pub struct NestedSubScreen {
    label: String,
}

impl NestedSubScreen {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
        }
    }

    pub fn from_route(route: &NestedRoute) -> Self {
        match route {
            NestedRoute::A => Self::new("Экран A"),
            NestedRoute::B => Self::new("Экран B"),
            NestedRoute::C => Self::new("Экран C"),
        }
    }
}

impl LifecycleObserver for NestedSubScreen {}

impl Component for NestedSubScreen {
    type State = ();
    type Message = RootMsg;

    fn render(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<Self::Message>) {
        let c = &Theme::current_from_ui(ui).colors;

        Column::new().show(ui, dispatch, |ui, dispatch| {
            Text::new(self.label.to_string())
                .text_color(c.on_secondary)
                .modifier(Modifier::new().padding(12.0).background(c.secondary))
                .render(ui, dispatch);

            Spacer::new(8.0).render(ui, dispatch);

            Text::new("Нажмите системную кнопку Back для возврата")
                .modifier(Modifier::new().padding(8.0))
                .render(ui, dispatch);

            Spacer::new(8.0).render(ui, dispatch);

            Button::new("← Назад (на уровень Nested)")
                .on_click(RootMsg::Back)
                .theme_colors(c.primary)
                .text_color(c.on_primary)
                .modifier(Modifier::new().fill_max_width().padding(8.0))
                .render(ui, dispatch);
        });
    }

    fn handle(&mut self, _msg: Self::Message) {}

    fn state(&self) -> &Self::State {
        &()
    }
}
