//! NestedScreen — экран с вложенной навигацией (ChildStack внутри).
//!
//! Обработка Back идёт через прямой вызов `handle_back()` из
//! `RootComponent::on_back()`, а не через BackDispatcher, чтобы избежать
//! проблем с временем жизни raw pointer (callback в BackDispatcher переживает
//! NestedScreen при pop из корневого стека, что приводит к SIGSEGV).
//!
//! - если есть вложенный экран → pop внутри (Handled)
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
///
/// Обработка Back — через прямой вызов `handle_back()` из RootComponent.
/// Не использует BackDispatcher, поэтому нет unsafe указателей.
pub struct NestedScreen {
    /// Вложенный стек: Route — ключ, NestedSubScreen — компонент.
    pub stack: ChildStack<Route, NestedSubScreen>,
}

impl NestedScreen {
    pub fn new() -> Self {
        Self {
            stack: ChildStack::new(),
        }
    }

    /// Обработать BackPressed — вызывается из RootComponent::on_back().
    /// Возвращает true если Back обработан (pop внутри вложенного стека), false если нет.
    pub fn handle_back(&mut self) -> bool {
        if self.stack.is_empty() {
            return false;
        }
        self.stack.pop();
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
