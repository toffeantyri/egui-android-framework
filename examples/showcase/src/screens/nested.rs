//! NestedScreen — экран с вложенной навигацией (ChildStack внутри).
//!
//! Демонстрирует обработку BackPressed через BackDispatcher.
//! Вложенный ChildStack управляет дочерними экранами NestedA/B/C.
//! При BackPressed: BackDispatcher вызывает зарегистрированные callback'и.
//! NestedScreen регистрирует callback для обработки Back:
//! - если есть вложенный экран → pop внутри
//! - если нет → NotHandled (Root делает pop)

use egui_android_framework::{
    navigation::ChildStack,
    runtime::Dispatcher,
    ui::{
        containers::Column,
        modifier::{Modifier, ModifierDsl},
        widgets::{Button, Spacer, Text, Widget},
        UiWrapper,
    },
};

use crate::navigation::Route;
use crate::root_component::RootMsg;
use crate::screens::nested_sub::NestedSubScreen;

/// Экран с вложенной навигацией.
pub struct NestedScreen {
    /// Вложенный стек: Route — ключ, NestedSubScreen — компонент.
    stack: ChildStack<Route, NestedSubScreen>,
}

impl NestedScreen {
    pub fn new() -> Self {
        let stack = ChildStack::new();
        Self { stack }
    }

    /// Вызывается из RootComponent для обработки Back.
    /// Если есть вложенный экран — ChildStack делает pop.
    /// Если стек пуст — возвращает false (Root делает pop).
    pub fn handle_back(&mut self) -> bool {
        if self.stack.is_empty() {
            return false;
        }
        self.stack.on_back();
        true
    }

    pub fn render(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<RootMsg>) {
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
                    active.render(ui, dispatch);
                    Spacer::new(16.0).render(ui, dispatch);
                }

                // Кнопки для навигации во вложенные экраны
                Button::new("Перейти на Nested A")
                    .on_click(RootMsg::Navigate(Route::NestedA))
                    .modifier(Modifier::new().fill_max_width().padding(4.0))
                    .render(ui, dispatch);
                Button::new("Перейти на Nested B")
                    .on_click(RootMsg::Navigate(Route::NestedB))
                    .modifier(Modifier::new().fill_max_width().padding(4.0))
                    .render(ui, dispatch);
                Button::new("Перейти на Nested C")
                    .on_click(RootMsg::Navigate(Route::NestedC))
                    .modifier(Modifier::new().fill_max_width().padding(4.0))
                    .render(ui, dispatch);

                Spacer::new(16.0).render(ui, dispatch);
                Button::new("← Назад")
                    .on_click(RootMsg::Back)
                    .modifier(Modifier::new().fill_max_width().padding(8.0))
                    .render(ui, dispatch);
            });
    }
}

impl NestedScreen {
    /// Перейти на вложенный экран.
    pub fn push_sub(&mut self, route: Route) {
        let sub = NestedSubScreen::from_route(&route);
        self.stack.push(route, sub);
    }

    /// Проверить, пуст ли вложенный стек.
    pub fn is_sub_stack_empty(&self) -> bool {
        self.stack.is_empty()
    }
}
