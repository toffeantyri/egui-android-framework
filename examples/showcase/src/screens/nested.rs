//! NestedScreen — экран с вложенной навигацией (ChildStack внутри).
//!
//! Демонстрирует обработку BackPressed через BackDispatcher.
//! При создании принимает ComponentContext и регистрирует callback
//! в BackDispatcher для обработки Back во вложенном стеке.
//!
//! - если есть вложенный экран → pop внутри (Handled)
//! - если нет → NotHandled (Root делает pop)

use egui_android_framework::{
    core::{BackCallback, ComponentContext},
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

/// Оборачивает `*mut ChildStack` в тип, реализующий `Send`.
///
/// Используется для передачи сырого указателя во вложение BackCallback.
/// SAFETY: указатель корректен, пока жив родительский NestedScreen.
struct RawStack(*mut ChildStack<Route, NestedSubScreen>);
unsafe impl Send for RawStack {}

impl RawStack {
    fn make_callback(self) -> Box<dyn FnMut() -> bool + Send + 'static> {
        // Мы уже реализовали Send для RawStack.
        // Сохраняем RawStack в Box, чтобы closure захватывал
        // опосредованно, а не напрямую `*mut`, который !Send.
        let inner = Box::new(self);
        Box::new(move || {
            let stack = unsafe { &mut *inner.0 };
            if stack.is_empty() {
                false
            } else {
                stack.pop();
                true
            }
        })
    }
}

impl NestedScreen {
    /// Создать NestedScreen и зарегистрировать обработчик Back.
    ///
    /// Принимает `ComponentContext` чтобы зарегистрировать callback
    /// в BackDispatcher. Callback перехватывает Back если вложенный
    /// стек не пуст — делает pop внутри.
    pub fn new(ctx: &mut ComponentContext<RootMsg, (), crate::app::AppState>) -> Self {
        let stack = ChildStack::new();

        // Регистрируем callback в BackDispatcher ДО создания Self,
        // чтобы избежать borrow conflict.
        // SAFETY: callback живёт пока жив NestedScreen.
        // Вызывается только из ComponentContext::on_back().
        let raw_stack = RawStack(
            &stack as *const ChildStack<Route, NestedSubScreen>
                as *mut ChildStack<Route, NestedSubScreen>,
        );

        ctx.back_dispatcher.register(BackCallback {
            priority: 50,
            handler: raw_stack.make_callback(),
        });

        Self { stack }
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
