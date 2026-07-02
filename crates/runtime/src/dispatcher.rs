//! Диспетчер сообщений — абстракция, скрывающая канал от View.
//!
//! View не знает про `std::sync::mpsc::Sender`. View видит только метод
//! [`Dispatcher::dispatch()`] — декларативное действие "отправить сообщение".
//!
//! Создаётся в [`Application::frame()`] и живёт один кадр.
//! После рендера все накопившиеся сообщения drain'ятся из `Receiver`
//! и передаются в [`Component::handle()`].
//!
//! # Пример
//!
//! ```ignore
//! let (dispatcher, receiver) = Dispatcher::new();
//! component.render(ui, &dispatcher);
//! while let Ok(msg) = receiver.try_recv() {
//!     component.handle(msg);
//! }
//! ```

use std::sync::mpsc;

/// Диспетчер сообщений для View.
///
/// View вызывает [`Dispatcher::dispatch()`] в момент события (клик, переключение и т.д.),
/// не заботясь о том, как сообщение будет доставлено до [`Component::handle()`].
///
/// Внутри использует `std::sync::mpsc::Sender`. Канал — FIFO, порядок сообщений
/// сохраняется.
///
/// # Generic
///
/// * `M` — тип сообщения приложения (например, `CounterMsg`).
pub struct Dispatcher<M> {
    tx: mpsc::Sender<M>,
}

impl<M> Dispatcher<M> {
    /// Создать пару `(Dispatcher, Receiver)`.
    ///
    /// `Dispatcher` отдаётся View для отправки сообщений.
    /// `Receiver` используется в `frame()` для сбора сообщений после render.
    pub fn new() -> (Self, mpsc::Receiver<M>) {
        let (tx, rx) = mpsc::channel();
        (Self { tx }, rx)
    }

    /// Отправить сообщение из View.
    ///
    /// Вызывается в момент события (клик, изменение текста и т.д.).
    /// Сообщение попадает в FIFO-очередь, откуда будет прочитано
    /// после завершения render.
    ///
    /// Ошибка отправки игнорируется — если Receiver дропнут,
    /// приложение уже завершается.
    pub fn dispatch(&self, msg: M) {
        let _ = self.tx.send(msg);
    }
}

impl<M> Clone for Dispatcher<M> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}
