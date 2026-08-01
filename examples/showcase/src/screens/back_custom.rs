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
            // Обработано (не-навигационное сообщение) — pop не нужен.
            return BackAction::Handled;
        }
        log::error!("BackCustomScreen::handle_dyn: ожидался RootMsg");
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

    fn handle(&mut self, msg: Self::Message, _ctx: &mut ComponentContext) {
        match msg {
            // RootMsg::Back перехватывается в app.rs централизованно:
            // downcast::<RootMsg>() → Ok → handle_msg → on_back → handle_back.
            // Не вызываем handle_back здесь — иначе будет двойной вызов.
            RootMsg::Back => {}
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
    use crate::navigation_host::RootMsg;
    use egui_android_framework::core::{BackAction, ComponentContext};

    /// handle_back — единая точка: первый вызов перехватывает Back.
    #[test]
    fn first_back_intercepts() {
        let mut screen = BackCustomScreen::new();
        let mut ctx = ComponentContext::new();
        assert_eq!(screen.handle_back(&mut ctx), BackAction::Handled);
        assert_eq!(screen.bg, BgColor::Green);
    }

    /// Второй вызов handle_back — Propagate (цвет уже зелёный).
    #[test]
    fn second_back_propagates() {
        let mut screen = BackCustomScreen::new();
        screen.bg = BgColor::Green;
        let mut ctx = ComponentContext::new();
        assert_eq!(screen.handle_back(&mut ctx), BackAction::Propagate);
    }

    /// handle() для RootMsg::Back НЕ вызывает handle_back.
    ///
    /// RootMsg::Back перехватывается в app.rs централизованно
    /// (downcast::<RootMsg>() → Ok → handle_msg → on_back → handle_back).
    /// handle() не должен дублировать этот вызов.
    #[test]
    fn handle_msg_back_does_not_call_handle_back() {
        let mut screen = BackCustomScreen::new();
        let mut ctx = ComponentContext::new();
        screen.handle(RootMsg::Back, &mut ctx);
        assert_eq!(
            screen.bg,
            BgColor::Blue,
            "handle() для Back НЕ должен вызывать handle_back — цвет не меняется"
        );
    }

    /// handle_dyn(RootMsg::Back) — запасной путь, если сообщение
    /// не перехвачено в app.rs. Тоже не должен вызывать handle_back.
    ///
    /// Возвращает Handled (сообщение обработано через handle()),
    /// но handle_back НЕ вызывается — это делает on_back().
    #[test]
    fn handle_dyn_back_does_not_call_handle_back() {
        let mut screen = BackCustomScreen::new();
        let mut ctx = ComponentContext::new();

        let action = screen.handle_dyn(Box::new(RootMsg::Back), &mut ctx);

        // handle_dyn вызывает handle() (который ничего не делает для Back),
        // затем возвращает Handled. handle_back НЕ вызывается.
        assert_eq!(
            action,
            BackAction::Handled,
            "handle_dyn для RootMsg::Back возвращает Handled"
        );
        assert_eq!(
            screen.bg,
            BgColor::Blue,
            "handle_dyn НЕ вызывает handle_back — bg не изменился"
        );
    }

    /// Полная цепочка: handle() → handle_back.
    /// Проверяет, что handle_back вызывается ровно 1 раз
    /// (handle() его не вызывает, только симулированный on_back).
    #[test]
    fn full_chain_handle_back_called_once() {
        let mut screen = BackCustomScreen::new();
        let mut ctx = ComponentContext::new();

        // Шаг 1: handle() — симуляция app.rs → handle_msg(RootMsg::Back)
        screen.handle(RootMsg::Back, &mut ctx);
        assert_eq!(screen.bg, BgColor::Blue, "handle() не меняет bg");

        // Шаг 2: симуляция on_back → ChildStack::on_back → handle_back
        let action = screen.handle_back(&mut ctx);
        assert_eq!(action, BackAction::Handled);
        assert_eq!(screen.bg, BgColor::Green, "handle_back переключил цвет");
    }
}
