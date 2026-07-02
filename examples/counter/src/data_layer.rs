//! Data layer для примера счётчика.
//!
//! Получает команды, пишет результат напрямую в StateStore.
//! После каждого изменения отправляет () в notify_tx —
//! UiNotifier в главном цикле подхватит и вызовет wake.

use egui_android_framework::runtime::StateStore;

use crate::msg::{CounterState, Msg};
use std::sync::mpsc;

/// Фоновая задача: получает команды, обновляет StateStore.
pub fn data_layer_worker(
    cmd_rx: mpsc::Receiver<Msg>,
    store: StateStore<CounterState>,
    notify_tx: mpsc::Sender<()>,
) {
    loop {
        match cmd_rx.recv() {
            Ok(Msg::Increment) => {
                store.update(|state| {
                    state.count = state.count.wrapping_add(1);
                    log::info!("DataLayer: count -> {}", state.count);
                });
                // Уведомить Runtime об изменении
                let _ = notify_tx.send(());
            }
            Err(_) => {
                log::info!("DataLayer: канал закрыт, завершаемся");
                break;
            }
        }
    }
}
