//! BackCustomScreen — экран, где Back меняет цвет фона вместо pop.
//!
//! Демонстрирует кастомную обработку Back: при нажатии переключается
//! цвет фона между синим и зелёным. Back не делает pop — RootComponent
//! возвращается на Home только при повторном Back (через back_fallback).

use egui_android_framework::core::{
    BackAction, Component as UiComponent, ComponentContext, ComponentNode, LifecycleObserver,
    UiWrapper,
};
use egui_android_framework::runtime::Dispatcher;
use egui_android_framework::ui::{
    containers::Column,
    modifier::{Modifier, ModifierDsl},
    theme::Theme,
    widgets::{Button, Spacer, Text, Widget},
};

use crate::navigation_host::RootMsg;

/// Состояние фона: два цвета для переключения.
#[derive(Clone, Debug, PartialEq)]
enum BgColor {
    Blue,
    Green,
}

pub struct BackCustomScreen {
    bg: BgColor,
}

impl BackCustomScreen {
    pub fn new() -> Self {
        Self { bg: BgColor::Blue }
    }
}

impl LifecycleObserver for BackCustomScreen {}

impl ComponentNode for BackCustomScreen {
    fn render(
        &self,
        ui: &mut UiWrapper,
        dispatch: &::egui_android_framework::runtime::DynDispatcher,
        ctx: &ComponentContext,
    ) {
        let typed = dispatch.wrap::<RootMsg>();
        UiComponent::render(self, ui, &typed, ctx);
    }

    fn handle_dyn(
        &mut self,
        msg: Box<dyn std::any::Any + Send>,
        ctx: &mut ComponentContext,
    ) -> BackAction {
        if let Ok(typed) = msg.downcast::<RootMsg>() {
            UiComponent::handle(self, *typed, ctx);
        } else {
            log::error!("BackCustomScreen::handle_dyn: ожидался RootMsg");
        }
        BackAction::Propagate
    }

    /// Кастомная обработка Back: переключает цвет фона.
    /// Первый вызов — переключение (Handled), второй — Propagate.
    fn handle_back(&mut self, _ctx: &mut ComponentContext) -> BackAction {
        match self.bg {
            BgColor::Blue => {
                self.bg = BgColor::Green;
                BackAction::Handled
            }
            BgColor::Green => BackAction::Propagate,
        }
    }

    fn save_state(&self) -> Option<Box<dyn std::any::Any + Send>> {
        None
    }
    fn restore_state(&mut self, _state: Box<dyn std::any::Any + Send>) {}

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl UiComponent for BackCustomScreen {
    type State = ();
    type Message = RootMsg;

    fn render(
        &self,
        ui: &mut UiWrapper,
        dispatch: &Dispatcher<Self::Message>,
        _ctx: &ComponentContext,
    ) {
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
                    .theme_colors(c.primary)
                    .text_color(c.on_primary)
                    .modifier(Modifier::new().fill_max_width().padding(8.0))
                    .render(ui, dispatch);
            });
    }

    fn handle(&mut self, msg: Self::Message, ctx: &mut ComponentContext) {
        match msg {
            RootMsg::Back => {
                self.handle_back(ctx);
            }
            _ => {}
        }
    }

    fn state(&self) -> &Self::State {
        &()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_back_intercepts() {
        let mut screen = BackCustomScreen::new();
        let mut ctx = ComponentContext::new();
        assert_eq!(screen.handle_back(&mut ctx), BackAction::Handled);
        assert_eq!(screen.bg, BgColor::Green);
    }

    #[test]
    fn second_back_propagates() {
        let mut screen = BackCustomScreen::new();
        screen.bg = BgColor::Green;
        let mut ctx = ComponentContext::new();
        assert_eq!(screen.handle_back(&mut ctx), BackAction::Propagate);
    }

    #[test]
    fn handle_msg_back_delegates_to_handle_back() {
        let mut screen = BackCustomScreen::new();
        let mut ctx = ComponentContext::new();
        screen.handle(RootMsg::Back, &mut ctx);
        // Первый вызов перехватывает
        assert_eq!(screen.bg, BgColor::Green);
    }
}
