//! NestedScreen — экран с вложенной навигацией (ChildStack внутри).
//!
//! Использует отдельный enum NestedRoute для вложенных экранов.
//! Обработка Back идёт через прямой вызов `handle_back()` из
//! `RootComponent::on_back()`, а не через BackDispatcher, чтобы избежать
//! проблем с временем жизни raw pointer.

use egui_android_framework::core::{ComponentNode, LifecycleObserver, UiWrapper};
use egui_android_framework::navigation::ChildStack;
use egui_android_framework::runtime::{Dispatcher, DynDispatcher};
use egui_android_framework::ui::{
    containers::Column,
    modifier::{Modifier, ModifierDsl},
    theme::Theme,
    widgets::{Button, Spacer, Text, Widget},
};

use crate::navigation::{NavigableRoute, NestedRoute};
use crate::navigation_host::RootMsg;
use crate::screens::nested_sub::NestedSubScreen;

/// Экран с вложенной навигацией.
pub struct NestedScreen {
    /// Вложенный стек: NestedRoute — ключ, NestedSubScreen — компонент.
    pub stack: ChildStack<NestedRoute>,
}

impl NestedScreen {
    pub fn new() -> Self {
        Self {
            stack: ChildStack::new(),
        }
    }

    /// Перейти на вложенный экран.
    pub fn push_sub(&mut self, route: NestedRoute) {
        let sub = NestedSubScreen::from_route(&route);
        self.stack.push(route, Box::new(sub));
    }

    /// Проверить, пуст ли вложенный стек.
    pub fn is_sub_stack_empty(&self) -> bool {
        self.stack.is_empty()
    }
}

impl LifecycleObserver for NestedScreen {}

impl ComponentNode for NestedScreen {
    fn render(&self, ui: &mut UiWrapper, uidynmsg_tx: &DynDispatcher) {
        let typed: Dispatcher<RootMsg> = uidynmsg_tx.wrap();
        let dispatch = &typed;

        let c = &Theme::current_from_ui(ui).colors;

        Column::new()
            .scrollable()
            .show(ui, dispatch, |ui, dispatch| {
                Text::new("Вложенная навигация")
                    .modifier(Modifier::new().padding(8.0))
                    .render(ui, dispatch);
                Spacer::new(8.0).render(ui, dispatch);

                Text::new("Нажмите кнопку для перехода на вложенный экран.")
                    .modifier(Modifier::new().padding(8.0))
                    .render(ui, dispatch);

                Spacer::new(16.0).render(ui, dispatch);

                // Если есть активный вложенный экран — показываем его
                if let Some(active) = self.stack.active() {
                    Text::new("Текущий экран:")
                        .modifier(Modifier::new().padding(8.0))
                        .render(ui, dispatch);
                    active.render(ui, uidynmsg_tx);
                    Spacer::new(16.0).render(ui, dispatch);
                }

                // Кнопки для навигации во вложенные экраны.
                // Шлём RootMsg::NavigateNested — RootComponent перенаправит
                // в NestedScreen::push_sub.
                Button::new("Перейти на Nested A")
                    .on_click(RootMsg::Navigate(NavigableRoute::Nested(NestedRoute::A)))
                    .theme_colors(c.secondary)
                    .text_color(c.on_secondary)
                    .modifier(Modifier::new().fill_max_width().padding(4.0))
                    .render(ui, dispatch);
                Button::new("Перейти на Nested B")
                    .on_click(RootMsg::Navigate(NavigableRoute::Nested(NestedRoute::B)))
                    .theme_colors(c.secondary)
                    .text_color(c.on_secondary)
                    .modifier(Modifier::new().fill_max_width().padding(4.0))
                    .render(ui, dispatch);
                Button::new("Перейти на Nested C")
                    .on_click(RootMsg::Navigate(NavigableRoute::Nested(NestedRoute::C)))
                    .theme_colors(c.secondary)
                    .text_color(c.on_secondary)
                    .modifier(Modifier::new().fill_max_width().padding(4.0))
                    .render(ui, dispatch);

                Spacer::new(16.0).render(ui, dispatch);
                Button::new("← Назад")
                    .on_click(RootMsg::Back)
                    .theme_colors(c.primary)
                    .text_color(c.on_primary)
                    .modifier(Modifier::new().fill_max_width().padding(8.0))
                    .render(ui, dispatch);
            });
    }

    fn handle_back(&mut self) -> bool {
        if self.stack.is_empty() {
            return false;
        }
        self.stack.pop();
        true
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn handle_dyn(&mut self, _msg: Box<dyn std::any::Any + Send>) {}
}
