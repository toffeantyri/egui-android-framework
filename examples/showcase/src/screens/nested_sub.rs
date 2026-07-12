//! Вложенный экран внутри NestedScreen.

use egui_android_framework::{
    core::{Component, LifecycleObserver},
    runtime::Dispatcher,
    ui::{
        containers::Column,
        modifier::{Modifier, ModifierDsl},
        theme::Theme,
        widgets::{Button, Spacer, Text, Widget},
        UiWrapper,
    },
};

use crate::navigation::Route;
use crate::root_component::RootMsg;

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

    pub fn from_route(route: &Route) -> Self {
        match route {
            Route::NestedA => Self::new("Экран A"),
            Route::NestedB => Self::new("Экран B"),
            Route::NestedC => Self::new("Экран C"),
            _ => Self::new("Неизвестный"),
        }
    }

    pub fn render(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<RootMsg>) {
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
}

impl LifecycleObserver for NestedSubScreen {}

impl Component for NestedSubScreen {
    type State = ();
    type Message = ();

    fn render(&self, _ui: &mut egui::Ui, _dispatch: &Dispatcher<Self::Message>) {}

    fn handle(&mut self, _msg: Self::Message) {}

    fn state(&self) -> &Self::State {
        &()
    }
}
