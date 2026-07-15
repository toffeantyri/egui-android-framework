//! Диспетчер сообщений — абстракция, скрывающая канал от View.
//!
//! View вызывает [`Dispatcher::dispatch()`] в момент события (клик, переключение и т.д.),
//! не заботясь о том, как сообщение будет доставлено до [`Component::handle()`].

use std::sync::mpsc;

// Трейт для полиморфного dispatch + clone
trait DispatcherImpl<M>: Send + Sync {
    fn dispatch(&self, msg: M);
    fn clone_box(&self) -> Box<dyn DispatcherImpl<M>>;
}

// Direct-реализация
struct DirectImpl<M> {
    tx: mpsc::Sender<M>,
}

impl<M: 'static + Send> DispatcherImpl<M> for DirectImpl<M> {
    fn dispatch(&self, msg: M) {
        let _ = self.tx.send(msg);
    }
    fn clone_box(&self) -> Box<dyn DispatcherImpl<M>> {
        Box::new(DirectImpl {
            tx: self.tx.clone(),
        })
    }
}

// Wrapped-реализация
struct WrappedImpl {
    tx: mpsc::Sender<Box<dyn std::any::Any + Send>>,
}

impl<M: 'static + Send> DispatcherImpl<M> for WrappedImpl {
    fn dispatch(&self, msg: M) {
        let _ = self.tx.send(Box::new(msg));
    }
    fn clone_box(&self) -> Box<dyn DispatcherImpl<M>> {
        Box::new(WrappedImpl {
            tx: self.tx.clone(),
        })
    }
}

/// Диспетчер сообщений для View.
pub struct Dispatcher<M> {
    inner: Box<dyn DispatcherImpl<M>>,
}

impl<M: 'static + Send> Dispatcher<M> {
    /// Создать пару `(Dispatcher, Receiver)` с прямым каналом.
    pub fn new() -> (Self, mpsc::Receiver<M>) {
        let (tx, rx) = mpsc::channel();
        (
            Self {
                inner: Box::new(DirectImpl { tx }),
            },
            rx,
        )
    }

    /// Отправить сообщение из View.
    pub fn dispatch(&self, msg: M) {
        self.inner.dispatch(msg);
    }

    /// Создать Dispatcher, упаковывающий M в DynDispatcher.
    #[doc(hidden)]
    pub fn wrap_from(dyn_dispatcher: &crate::DynDispatcher) -> Self {
        Self {
            inner: Box::new(WrappedImpl {
                tx: dyn_dispatcher.tx.clone(),
            }),
        }
    }
}

impl<M: 'static + Send> Clone for Dispatcher<M> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone_box(),
        }
    }
}
