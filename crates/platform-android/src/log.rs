use std::time::{SystemTime, UNIX_EPOCH};

const MAX_COUNTERS: usize = 16;

/// Простейший хеш строки для выбора слота счётчика.
fn slot_id(counter_id: &str) -> usize {
    let hash: u64 = counter_id
        .bytes()
        .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
    (hash as usize) % MAX_COUNTERS
}

/// Логировать с троттлингом — не чаще раза в `interval_secs`.
///
/// Каждый вызывающий сайт должен использовать свой уникальный `counter_id`.
/// Идентификатор может быть любым — например, краткое описание лога.
pub(crate) fn log_with_throttling<F: Fn() -> String>(interval_secs: u64, message_fn: F) {
    log_with_throttling_id("__default__", interval_secs, message_fn);
}

/// Логировать с троттлингом, используя явный строковый идентификатор счётчика.
///
/// `counter_id` должен быть уникальным для каждого места вызова.
pub(crate) fn log_with_throttling_id<F: Fn() -> String>(
    counter_id: &str,
    interval_secs: u64,
    message_fn: F,
) {
    static mut LAST_LOG_SEC: [u64; MAX_COUNTERS] = [0; MAX_COUNTERS];

    let idx = slot_id(counter_id);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // SAFETY: статический массив используется только
    // для throttling логов в однопоточном JNI-контексте.
    let should_log = unsafe {
        let prev = LAST_LOG_SEC[idx];
        if now >= prev + interval_secs {
            LAST_LOG_SEC[idx] = now;
            true
        } else {
            false
        }
    };

    if should_log {
        log::info!("{}", message_fn());
    } else {
        log::trace!("log_with_throttling_id[{}]: throttled", counter_id);
    }
}
