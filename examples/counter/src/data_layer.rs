//! Data layer для примера счётчика.
//!
//! Получает команды, пишет результат напрямую в StateStore.
//! После каждого изменения отправляет () в statechanged_tx —
//! UiNotifier в главном цикле подхватит и вызовет wake.

use egui_android_framework::runtime::StateStore;

use crate::msg::{CounterState, Msg};
use std::sync::mpsc;

/// Фоновая задача: получает команды, обновляет StateStore.
pub fn data_layer_worker(
    datacmd_rx: mpsc::Receiver<Msg>,
    store: StateStore<CounterState>,
    statechanged_tx: mpsc::Sender<()>,
) {
    loop {
        match datacmd_rx.recv() {
            Ok(Msg::Increment) => {
                store.update(|state| {
                    state.count = state.count.wrapping_add(1);
                    log::info!("DataLayer: count -> {}", state.count);
                });
                let _ = statechanged_tx.send(());
            }
            Ok(Msg::ToggleTheme) => {
                // ToggleTheme обрабатывается в Application::frame()
                log::info!("DataLayer: ToggleTheme (игнорируется в data layer)");
            }
            Err(_) => {
                log::info!("DataLayer: канал закрыт, завершаемся");
                break;
            }
        }
    }
}
