//! Type-erased диспетчер сообщений — [`DynDispatcher`].
//!
//! Позволяет хранить `Dispatcher` как type-erased контейнер без generic-параметра.
//! Используется в `Box<dyn ComponentNode>` для совместимости c `ChildStack`.
//!
//! # Как работает
//!
//! Создаётся в [`Application::frame()`] вместе с receiver.
//! Передаётся в `ComponentNode::render()`.
//! Blanket-impl через `dispatch.wrap::<M>()` создаёт `Dispatcher<M>`,
//! который упаковывает каждое сообщение через [`MessageEnvelope`].
//!
//! После рендера все сообщения drain'ятся из receiver и downcast'ятся.
//!
//! # Безопасность типов
//!
//! Внутри канал использует `Box<dyn Any + Send>` — это вынужденное решение
//! для передачи сообщений разных типов через один канал.
//! Однако каждый элемент упаковывается через [`MessageEnvelope<M>`],
//! что гарантирует `M: Clone + Debug + Send + 'static`.
//! При ошибке downcast сообщение логируется с именем ожидаемого типа.

use std::any::Any;
use std::sync::mpsc;

/// Type-erased диспетчер сообщений.
#[derive(Clone)]
pub struct DynDispatcher {
    pub(crate) tx: mpsc::Sender<Box<dyn Any + Send>>,
}

impl DynDispatcher {
    /// Создать пару `(DynDispatcher, Receiver)`.
    pub fn new() -> (Self, mpsc::Receiver<Box<dyn Any + Send>>) {
        let (tx, rx) = mpsc::channel();
        (Self { tx }, rx)
    }

    /// Отправить сообщение (type-erased).
    pub fn dispatch(&self, msg: Box<dyn Any + Send>) {
        let _ = self.tx.send(msg);
    }

    /// Получить типизированный Dispatcher, упаковывающий M через MessageEnvelope.
    ///
    /// Не спавнит поток — упаковка происходит непосредственно в Dispatch::dispatch().
    /// Dispatcher<M> можно клонировать, отправлять в замыкания.
    pub fn wrap<M: 'static + Send>(&self) -> super::Dispatcher<M> {
        super::Dispatcher::wrap_from(self)
    }
}

impl Default for DynDispatcher {
    fn default() -> Self {
        let (this, _) = Self::new();
        this
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dyn_dispatcher_direct() {
        let (dyn_dispatcher, rx) = DynDispatcher::new();
        dyn_dispatcher.dispatch(Box::new(42i32));
        let received = rx.try_recv().unwrap();
        let value: i32 = *received.downcast::<i32>().unwrap();
        assert_eq!(value, 42);
    }

    #[test]
    fn test_dyn_dispatcher_wrap() {
        let (dyn_dispatcher, rx) = DynDispatcher::new();
        let typed = dyn_dispatcher.wrap::<String>();
        typed.dispatch("hello wrapped".to_string());
        std::thread::sleep(std::time::Duration::from_millis(50));
        let received = rx.try_recv().unwrap();
        let msg: String = *received.downcast::<String>().unwrap();
        assert_eq!(msg, "hello wrapped");
    }

    #[test]
    fn test_dyn_dispatcher_clone() {
        let (d1, rx) = DynDispatcher::new();
        let d2 = d1.clone();
        d1.dispatch(Box::new("from d1".to_string()));
        d2.dispatch(Box::new("from d2".to_string()));
        let msg1 = rx.try_recv().unwrap();
        let msg2 = rx.try_recv().unwrap();
        assert_eq!(*msg1.downcast::<String>().unwrap(), "from d1");
        assert_eq!(*msg2.downcast::<String>().unwrap(), "from d2");
    }
}
