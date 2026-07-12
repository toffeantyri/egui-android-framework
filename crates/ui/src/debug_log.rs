//! Переиспользуемый модуль для отладочного логирования с ограничением по частоте.
//!
//! Модуль оставлен для будущей отладки layout-проблем.
//! Сознательно не удаляется — все элементы могут быть востребованы.

#![allow(dead_code)]

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

const SLOTS: usize = 16;
static LAST_LOGS: [AtomicU64; SLOTS] = [
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
];

/// Раз в `throttle_ms` миллисекунд пишет в `log::info!` строку из замыкания.
/// `name` — ключ (например "Text"), по нему вычисляется слот.
pub fn throttled_log(name: &str, throttle_ms: u64, fmt: impl FnOnce() -> String) {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let slot = name.bytes().fold(0usize, |a, b| a.wrapping_add(b as usize)) % SLOTS;
    let last = LAST_LOGS[slot].load(Ordering::Relaxed);

    if now.saturating_sub(last) > throttle_ms || last == 0 {
        LAST_LOGS[slot].store(now, Ordering::Relaxed);
        log::info!("[{}] {}", name, fmt());
    }
}
