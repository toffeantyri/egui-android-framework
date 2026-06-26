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

use std::sync::mpsc;

use crate::egui_subscriber::AndroidWakeHandle;

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
