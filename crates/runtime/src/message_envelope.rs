//! Типобезопасная обёртка для сообщений, передаваемых через DynDispatcher.
//!
//! [`MessageEnvelope<M>`] — newtype над `M`, который гарантирует,
//! что сообщение реализует `Clone + Debug + Send + 'static`.
//!
//! # Зачем
//!
//! `DynDispatcher` использует `Box<dyn Any + Send>` для передачи сообщений
//! разных типов через один канал. `MessageEnvelope` добавляет:
//! - Статическую проверку: `M: Clone + Debug + Send + 'static`
//! - Возможность логирования типа сообщения при ошибках downcast
//! - Единый контракт для всех сообщений в системе

use std::fmt::Debug;

/// Типобезопасная обёртка сообщения.
///
/// Единственный способ создать — через `MessageEnvelope::new(msg)`.
/// Единственный способ прочитать — через `MessageEnvelope::into_inner(self)`.
///
/// # Гарантии
///
/// - `M: Clone + Debug + Send + 'static` — проверяется на этапе компиляции
/// - Никакой другой код не может создать `MessageEnvelope` без этих bounds
#[derive(Clone, Debug)]
pub struct MessageEnvelope<M>
where
    M: Clone + Debug + Send + 'static,
{
    inner: M,
}

impl<M> MessageEnvelope<M>
where
    M: Clone + Debug + Send + 'static,
{
    /// Создать новый конверт с сообщением.
    pub fn new(msg: M) -> Self {
        Self { inner: msg }
    }

    /// Извлечь сообщение.
    pub fn into_inner(self) -> M {
        self.inner
    }

    /// Получить ссылку на сообщение.
    pub fn as_ref(&self) -> &M {
        &self.inner
    }
}
