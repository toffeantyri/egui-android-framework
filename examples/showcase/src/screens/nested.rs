//! NestedScreen — экран с вложенной навигацией (ChildStack внутри).
//!
//! Демонстрирует обработку BackPressed через ChildStack::register_back_handler.
//! Вложенный ChildStack управляет дочерними экранами NestedA/B/C.
//! При BackPressed: если есть активный вложенный экран — pop внутри,
//! иначе Back уходит в RootComponent (pop на уровень Home).

use std::sync::mpsc;

use egui_android_framework::{
    navigation::{BackHandling, ChildStack},
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
    /// Receiver для BackPressed от ChildStack.
    #[allow(dead_code)]
    back_rx: mpsc::Receiver<()>,
}

impl NestedScreen {
    pub fn new() -> Self {
        let (back_tx, back_rx) = mpsc::channel();
        let mut stack = ChildStack::new();
        // Регистрируем обработчик Back в нашем стеке
        stack.register_back_handler(back_tx);
        Self { stack, back_rx }
    }

    /// Обработать BackPressed — вызывается из RootComponent.
    ///
    /// ChildStack сам решает: если есть вложенный компонент с handler —
    /// отправляет ему, если нет — делает pop внутри своего стека.
    /// В любом случае возвращаем Handled — RootComponent не должен делать pop,
    /// потому что нас может быть несколько в стеке RootComponent.
    /// Если мы сами опустели — следующий BackRootComponent передаст снова,
    /// и тогда on_back() вернёт NotHandled.
    pub fn on_back(&mut self) -> BackHandling {
        // ChildStack уровня 2 сам разруливает:
        // - если есть handler у вложенного → Handled
        // - если нет → pop внутри (стек уменьшается)
        // - если пуст → NotHandled
        let handling = self.stack.on_back();
        if handling == BackHandling::Handled {
            return BackHandling::Handled;
        }
        // Вложенный стек пуст или не обработал — ChildStack не сделал pop.
        //               значит надо pop на уровень Root (Home).
        BackHandling::NotHandled
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
}
