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
//! - `handle_back` — встроенная поддержка BackPressed (как в Decompose)
//! - `save_state` / `restore_state` — сохранение состояния для пересоздания Activity
//!
//! # Blanket-impl
//!
//! Любой тип, реализующий [`Component`], автоматически реализует [`ComponentNode`]:
//!
//! ```ignore
//! impl<T: Component> ComponentNode for T { ... }
//! ```
//!
//! Таким образом, экраны реализуют `Component`, а `ChildStack` хранит их как
//! `Box<dyn ComponentNode>`.

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
    /// `dispatch.typed::<Self::Message>()` и передать его в View-функцию.
    fn render(&self, ui: &mut UiWrapper, dispatch: &DynDispatcher);

    /// Обработать type-erased сообщение от View.
    ///
    /// Реализация должна downcast'ить `msg` в `Self::Message` и вызвать `handle()`.
    fn handle_dyn(&mut self, msg: Box<dyn std::any::Any + Send>);

    /// Обработать BackPressed. Возвращает `true`, если Back перехвачен.
    ///
    /// По умолчанию — `false` (Back не обработан, передаётся дальше).
    /// Экран может переопределить для кастомной обработки Back.
    fn handle_back(&mut self) -> bool {
        false
    }

    /// Сохранить состояние компонента для восстановления после пересоздания.
    ///
    /// По умолчанию — `None` (состояние не сохраняется).
    /// Экран может вернуть любое `Send + 'static` значение (например, счётчик).
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

// Blanket-impl: любой Component автоматически становится ComponentNode.
impl<T> ComponentNode for T
where
    T: crate::Component,
    T::Message: 'static + Send,
{
    fn render(&self, ui: &mut UiWrapper, dispatch: &DynDispatcher) {
        let typed = dispatch.wrap::<T::Message>();
        crate::Component::render(self, ui, &typed);
    }

    fn handle_dyn(&mut self, msg: Box<dyn std::any::Any + Send>) {
        if let Ok(typed) = msg.downcast::<T::Message>() {
            crate::Component::handle(self, *typed);
        } else {
            log::error!(
                "ComponentNode::handle_dyn: ошибка типа сообщения — ожидался {}, получен неизвестный тип",
                std::any::type_name::<T::Message>()
            );
        }
    }

    fn handle_back(&mut self) -> bool {
        false
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
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

        fn render(&self, _ui: &mut UiWrapper, _dispatch: &Dispatcher<Self::Message>) {}

        fn handle(&mut self, msg: String) {
            self.handled.push(msg);
        }

        fn state(&self) -> &Self::State {
            &()
        }
    }

    #[test]
    fn test_component_node_blanket_impl() {
        let comp = TestComponent::default();
        let _node: Box<dyn ComponentNode> = Box::new(comp);
    }

    #[test]
    fn test_handle_dyn_downcasts() {
        let comp = TestComponent::default();
        let mut node: Box<dyn ComponentNode> = Box::new(comp);

        node.handle_dyn(Box::new("hello".to_string()));
        node.handle_dyn(Box::new("world".to_string()));

        // Достаём обратно, чтобы проверить состояние
        let comp = node.as_any_mut().downcast_mut::<TestComponent>().unwrap();
        assert_eq!(comp.handled, vec!["hello".to_string(), "world".to_string()]);
    }

    #[test]
    fn test_handle_back_default() {
        struct NoBack;
        impl LifecycleObserver for NoBack {}
        impl crate::Component for NoBack {
            type State = ();
            type Message = ();
            fn render(&self, _ui: &mut UiWrapper, _dispatch: &Dispatcher<Self::Message>) {}
            fn handle(&mut self, _msg: ()) {}
            fn state(&self) -> &Self::State {
                &()
            }
        }

        let mut node: Box<dyn ComponentNode> = Box::new(NoBack);
        assert!(!node.handle_back());
    }
}
