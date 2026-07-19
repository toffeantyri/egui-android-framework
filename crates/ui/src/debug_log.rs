//! Отладочное логирование layout-событий с ограничением по частоте (throttle).
//!
//! # Назначение
//!
//! Используется для диагностики layout-проблем в рантайме.
//! Вызов `throttled_log("T6", 1000, || format!(...))` пишет в logcat
//! не чаще раза в `throttle_ms` миллисекунд для каждого именованного слота.
//!
//! # Использование
//!
//! 1. Раскомментировать вызов `throttled_log` в нужном месте (например, text.rs).
//! 2. Собрать APK и запустить — логи пойдут в logcat с фильтром `egui-showcase`.
//!
//! # ВАЖНО: НЕ удалять этот модуль
//!
//! Модуль сознательно сохранён для будущей отладки layout-проблем.
//! Он отключён в production через `#![allow(dead_code)]` — не влияет на бинарник,
//! так как ни один крейт не импортирует его публично (`pub(crate)` в lib.rs).
//!
//! # Архитектурное примечание
//!
//! Модуль использует глобальный статический массив `LAST_LOGS: [AtomicU64; 16]`.
//! Формально это нарушает правило «запрещены глобальные статики для Sender'ов»,
//! но здесь статика используется **только** для throttle-логирования (чтение/запись u64),
//! а не для передачи сообщений или хранения состояния платформы.
//! Это **единственное** исключение из правила — и только потому, что модуль
//! является отладочным инструментом и не участвует в архитектурном потоке данных.
//! При рефакторинге platform-android (удаление глобальных статик) этот модуль НЕ трогать.

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
///
/// `name` — ключ (например "Text"), по нему вычисляется слот в статическом массиве.
/// Если с момента последнего логирования прошло меньше `throttle_ms` — вызов игнорируется.
///
/// # Пример
///
/// ```ignore
/// throttled_log("T6", 1000, || format!("uid={} text={:?}", uid, text));
/// ```
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
