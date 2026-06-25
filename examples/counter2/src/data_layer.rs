//! Data layer для примера счётчика.
//!
//! Фоновая задача: получает команды, изменяет состояние, шлёт события.

use crate::msg::{Evt, Msg};
use std::sync::mpsc;

/// Фоновая задача: получает команды, изменяет состояние, шлёт события.
pub fn data_layer_worker(cmd_rx: mpsc::Receiver<Msg>, evt_tx: mpsc::Sender<Evt>) {
    let mut count = 0u32;
    loop {
        match cmd_rx.recv() {
            Ok(Msg::Increment) => {
                count = count.wrapping_add(1);
                log::info!("DataLayer: count -> {count}");
                if evt_tx.send(Evt::CountUpdated(count)).is_err() {
                    log::info!("DataLayer: получатель отключён, завершаемся");
                    break;
                }
            }
            Err(_) => {
                log::info!("DataLayer: канал закрыт, завершаемся");
                break;
            }
        }
    }
}
