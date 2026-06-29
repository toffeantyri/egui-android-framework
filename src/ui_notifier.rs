//! Инфраструктурный `UiNotifier` — связывает Store и Runtime.
//!
//! Живёт в главном потоке (там же где egui::Context).
//! Вызывается синхронно из главного цикла — проверяет канал уведомлений
//! и при наличии сигнала вызывает request_repaint() + wake().
//!
//! Никаких фоновых потоков, никакого polling.
//!
//! Контракт:
//! - Знает только про egui::Context, AndroidWakeHandle и mpsc
//! - Не знает про Domain, Components, Reducer, Data Layer
//! - Вызывается Runtime, не бизнес-логикой

use std::sync::{mpsc, Arc};

/// Handle для пробуждения Android event loop.
///
/// Создаётся из замыкания, которое вызывает `AndroidApp::wake()`.
/// В тестах можно использовать заглушку.
///
/// # Пример
///
/// ```ignore
/// let wake = AndroidWakeHandle::new(move || { let _ = app.wake(); });
/// ```
pub struct AndroidWakeHandle {
    wake_fn: Arc<dyn Fn() + Send + Sync>,
}

impl Clone for AndroidWakeHandle {
    fn clone(&self) -> Self {
        Self {
            wake_fn: Arc::clone(&self.wake_fn),
        }
    }
}

impl AndroidWakeHandle {
    /// Создать новый handle.
    pub fn new(wake_fn: impl Fn() + Send + Sync + 'static) -> Self {
        Self {
            wake_fn: Arc::new(wake_fn),
        }
    }

    /// Пробудить Android event loop.
    ///
    /// Вызывает `MainEvent::Wake` в главном цикле `run()`.
    pub fn wake(&self) {
        (self.wake_fn)();
    }
}

/// Инфраструктурный уведомитель.
///
/// Создаётся в Runtime, хранит `Receiver<()>`.
/// Data layer отправляет `()` в канал после изменения состояния.
/// Runtime вызывает `check()` — если есть сигнал, вызывает repaint + wake.
pub struct UiNotifier {
    ctx: egui::Context,
    wake: Option<AndroidWakeHandle>,
    rx: mpsc::Receiver<()>,
}

impl UiNotifier {
    /// Создать уведомитель.
    ///
    /// `rx` — приёмник сигналов от data layer.
    /// Data layer отправляет `()` после каждого `store.update()`.
    pub fn new(
        ctx: egui::Context,
        wake: Option<AndroidWakeHandle>,
        rx: mpsc::Receiver<()>,
    ) -> Self {
        Self { ctx, wake, rx }
    }

    /// Проверить наличие сигнала об изменении состояния.
    ///
    /// Вызывается Runtime в главном цикле, без блокировки.
    /// Если сигнал есть — вызывает `request_repaint()` и `wake()`.
    pub fn check(&mut self) {
        let mut changed = false;
        while self.rx.try_recv().is_ok() {
            changed = true;
        }
        if changed {
            self.ctx.request_repaint();
            if let Some(ref w) = self.wake {
                w.wake();
            }
        }
    }
}
