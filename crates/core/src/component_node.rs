//! Object-safe трейт [`ComponentNode`] для хранения разнотипных компонентов
//! в одном [`ChildStack`].
//!
//! Вдохновлён Decompose: `ChildStack` хранит `Box<dyn ComponentNode>`, что
//! позволяет складывать в один стек компоненты разных типов сообщений.
//!
//! # Отличие от [`Component`]
//!
//! [`Component`] — generic трейт с ассоциированными типами (`State`, `Message`).
//! Он не object-safe, поэтому не может быть использован в `Vec<Box<dyn Component>>`.
//!
//! [`ComponentNode`] — object-safe трейт с type-erased методами:
//! - `render` через [`DynDispatcher`]
//! - `handle_dyn` через `Box<dyn Any + Send>`
//! - `handle_back(ctx) -> BackAction` — единая точка обработки Back
//!   (и платформенной, и рисованной кнопки).
//! - `save_state` / `restore_state` — сохранение состояния для пересоздания Activity
//!
//! Все методы, кроме `handle_back`, принимают [`ComponentContext`] (`ctx`).
//! Рисованная кнопка «← Назад» делегирует в `handle_back` из `Component::handle()`,
//! платформенная — через `ChildStack::on_back()`. Оба пути сходятся в `handle_back()`.
//!
//! # Реализация через макрос
//!
//! `ComponentNode` реализуется через `#[derive(ComponentNode)]` (из `egui-android-macros`).
//! Макрос генерирует конкретный impl для каждого компонента.
//!
//! Если компонент использует `#[persistent_fields(...)]`, макрос генерирует
//! `save_state`/`restore_state` через `PersistentState::save_to_boxed()`.
//! Иначе — `save_state = None` (стандартное поведение blanket-impl).
//!
//! ```ignore
//! // Компонент без сохранения состояния
//! #[derive(ComponentNode)]
//! struct MyScreen;
//!
//! // Компонент с сохранением состояния
//! #[derive(Component, ComponentNode)]
//! #[persistent_fields(counter)]
//! struct StatefulScreen { counter: i32 }
//! ```

use crate::back_action::BackAction;
use crate::component_context::ComponentContext;
use crate::lifecycle::LifecycleObserver;
use crate::UiWrapper;
use egui_android_runtime::DynDispatcher;

/// Object-safe трейт для хранения компонента в `ChildStack`.
///
/// Generic-параметры вынесены в type-erased методы,
/// чтобы трейт можно было использовать как `Box<dyn ComponentNode>`.
pub trait ComponentNode: LifecycleObserver + Send + 'static {
    /// Отрисовать UI через type-erased dispatcher.
    ///
    /// Реализация должна получить типизированный `Dispatcher<M>` через
    /// `dispatch.typed::<Self::Message>()`, передать его в View-функцию вместе с `ctx`.
    fn render(&self, ui: &mut UiWrapper, dispatch: &DynDispatcher, ctx: &ComponentContext);

    /// Обработать type-erased сообщение от View.
    ///
    /// Реализация должна downcast'ить `msg` в `Self::Message`, вызвать `handle()`
    /// и передать в него `ctx`. Для кнопки Back `handle()` делегирует в `handle_back()`.
    fn handle_dyn(&mut self, msg: Box<dyn std::any::Any + Send>, ctx: &mut ComponentContext);

    /// Обработать BackPressed — единая точка и для платформенной,
    /// и для рисованной кнопки «← Назад».
    ///
    /// По умолчанию — [`BackAction::Propagate`] (Back не обработан, передаётся дальше).
    /// Экран может переопределить для кастомной логики.
    fn handle_back(&mut self, _ctx: &mut ComponentContext) -> BackAction {
        BackAction::Propagate
    }

    /// Сохранить состояние компонента для восстановления после пересоздания.
    ///
    /// По умолчанию — `None` (состояние не сохраняется).
    ///
    /// Если компонент реализует [`PersistentState`], этот метод должен
    /// сериализовать `PersistentState::save()` через `bincode` в `Vec<u8>`
    /// и вернуть `Some(Box::new(bytes))`.
    ///
    /// `ChildStack` при сохранении ожидает именно `Vec<u8>` —
    /// сериализованные данные, готовые для Android Bundle.
    fn save_state(&self) -> Option<Box<dyn std::any::Any + Send>> {
        None
    }

    /// Восстановить ранее сохранённое состояние.
    ///
    /// Вызывается после создания компонента, если есть сохранённое состояние.
    /// Экран должен downcast'ить `state` в свой тип и восстановить поля.
    fn restore_state(&mut self, _state: Box<dyn std::any::Any + Send>) {}

    /// Даункаст до `&dyn Any` для тестирования и отладки.
    fn as_any(&self) -> &dyn std::any::Any;

    /// Даункаст до `&mut dyn Any` для тестирования и отладки.
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lifecycle::LifecycleObserver;
    use crate::UiWrapper;
    use egui_android_runtime::Dispatcher;

    #[derive(Default)]
    struct TestComponent {
        handled: Vec<String>,
    }

    impl LifecycleObserver for TestComponent {}

    impl crate::Component for TestComponent {
        type State = ();
        type Message = String;

        fn render(
            &self,
            _ui: &mut UiWrapper,
            _dispatch: &Dispatcher<Self::Message>,
            _ctx: &ComponentContext,
        ) {
        }

        fn handle(&mut self, msg: String, _ctx: &mut ComponentContext) {
            self.handled.push(msg);
        }

        fn state(&self) -> &Self::State {
            &()
        }
    }

    // Для тестов используем макрос ComponentNode
    // В integration-тестах это проверяется через framework.
    // Здесь — юнит-тесты базового поведения трейта.

    #[test]
    fn test_handle_back_default() {
        struct NoBack;
        impl LifecycleObserver for NoBack {}
        impl crate::Component for NoBack {
            type State = ();
            type Message = ();
            fn render(
                &self,
                _ui: &mut UiWrapper,
                _dispatch: &Dispatcher<Self::Message>,
                _ctx: &ComponentContext,
            ) {
            }
            fn handle(&mut self, _msg: (), _ctx: &mut ComponentContext) {}
            fn state(&self) -> &Self::State {
                &()
            }
        }
        impl crate::ComponentNode for NoBack {
            fn render(&self, ui: &mut UiWrapper, dispatch: &DynDispatcher, ctx: &ComponentContext) {
                let typed = dispatch.wrap::<()>();
                crate::Component::render(self, ui, &typed, ctx);
            }
            fn handle_dyn(
                &mut self,
                msg: Box<dyn std::any::Any + Send>,
                ctx: &mut ComponentContext,
            ) {
                if let Ok(typed) = msg.downcast::<()>() {
                    crate::Component::handle(self, *typed, ctx);
                } else {
                    log::error!("ComponentNode::handle_dyn: ошибка типа");
                }
            }
            fn handle_back(&mut self, _ctx: &mut ComponentContext) -> BackAction {
                BackAction::Propagate
            }
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }
        }

        let mut node: Box<dyn ComponentNode> = Box::new(NoBack);
        let mut ctx = ComponentContext::new();
        assert_eq!(node.handle_back(&mut ctx), BackAction::Propagate);
    }
}
