//! Data layer для примера счётчика.
//!
//! Фоновая задача: получает команды, изменяет состояние через StateStore.
//! Больше не использует mpsc для отправки событий обратно — пишет напрямую
//! в реактивный store, что вызывает request_repaint() + wake() автоматически.

use egui_android_framework::store::StateStore;

use crate::msg::{CounterState, Msg};
use std::sync::mpsc;

/// Фоновая задача: получает команды, изменяет состояние через StateStore.
pub fn data_layer_worker(cmd_rx: mpsc::Receiver<Msg>, store: StateStore<CounterState>) {
    loop {
        match cmd_rx.recv() {
            Ok(Msg::Increment) => {
                // Единственная точка изменения состояния — через store.update()
                // Это автоматически уведомит всех подписчиков (EguiRepaintSubscriber),
                // который вызовет request_repaint() и wake().
                store.update(|state| {
                    state.count = state.count.wrapping_add(1);
                    log::info!("DataLayer: count -> {}", state.count);
                });
            }
            Err(_) => {
                log::info!("DataLayer: канал закрыт, завершаемся");
                break;
            }
        }
    }
}
