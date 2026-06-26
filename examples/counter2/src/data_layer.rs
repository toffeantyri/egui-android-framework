//! Data layer для примера счётчика.
//!
//! Получает команды, пишет результат напрямую в StateStore.
//! Не шлёт события через mpsc — изменение состояния публикуется
//! через store.update(), что автоматически вызывает request_repaint() + wake().

use egui_android_framework::store::StateStore;

use crate::msg::{CounterState, Msg};
use std::sync::mpsc;

/// Фоновая задача: получает команды, обновляет StateStore.
pub fn data_layer_worker(cmd_rx: mpsc::Receiver<Msg>, store: StateStore<CounterState>) {
    loop {
        match cmd_rx.recv() {
            Ok(Msg::Increment) => {
                // Единственная точка изменения состояния.
                // После update() все подписчики (EguiRepaintSubscriber)
                // получат уведомление → request_repaint() → wake() → новый кадр.
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
