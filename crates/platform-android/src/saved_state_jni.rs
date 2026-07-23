//! JNI-мост для kill/restore процесса.
//!
//! Передаёт сериализованный `Vec<u8>` (SavedStack<C>) между
//! Rust `PlatformState` и Android `Bundle` через kotlin-бридж в `EguiActivity`.
//!
//! # Поток данных
//!
//! ```text
//! on_save_state() → PlatformState.set_saved_state(bytes)
//!   → nativeGetSavedState (JNI) → Bundle.putByteArray
//!
//! Bundle.getByteArray → nativeSetSavedState (JNI)
//!   → PlatformState.set_saved_state(bytes)
//!   → on_restore_state(Some(bytes))
//! ```
//!
//! # Проблема порядка инициализации
//!
//! `nativeSetSavedState` вызывается из `EguiActivity.onCreate()`.
//! `GLOBAL_PLATFORM_STATE` инициализируется в `run_with_backend()`.
//! Порядок этих вызовов не гарантирован (race condition между Kotlin и Rust).
//!
//! Решение: `PENDING_SAVED_STATE` — временный буфер. Если `nativeSetSavedState`
//! вызван до инициализации `GLOBAL_PLATFORM_STATE`, данные сохраняются в pending.
//! При `init_jni_platform_state()` pending-данные переносятся в PlatformState.
//!
//! # Именование JNI-функций
//!
//! JNI-функции используют формат `Java_<package>_<Class>_<method>`.
//! Package: `com.example.egui_android` (из AndroidManifest + EguiActivity).
//! Class: `EguiActivity`.

#![cfg(target_os = "android")]

use crate::platform_state::PlatformState;
use jni::sys::jbyteArray;
use jni::JNIEnv;
use std::sync::OnceLock;

/// Глобальная ссылка на PlatformState для доступа из JNI-функций.
///
/// Инициализируется в `run_with_backend()` после создания backend'а.
static GLOBAL_PLATFORM_STATE: OnceLock<PlatformState> = OnceLock::new();

/// Временный буфер для данных, пришедших через `nativeSetSavedState`
/// до инициализации `GLOBAL_PLATFORM_STATE`.
///
/// Устанавливается в `nativeSetSavedState`, читается и очищается
/// в `init_jni_platform_state()`.
static PENDING_SAVED_STATE: OnceLock<Vec<u8>> = OnceLock::new();

/// Зарегистрировать PlatformState для доступа из JNI.
///
/// Вызывается из `run_with_backend()` при каждом запуске.
/// Идемпотентен: если PlatformState уже зарегистрирован — обновляет
/// внутреннее состояние (insets, тема, JNI-указатели) но не паникует.
pub fn init_jni_platform_state(state: PlatformState) {
    // Проверяем pending-данные, пришедшие до инициализации
    if let Some(pending) = PENDING_SAVED_STATE.get() {
        log::info!(
            "JNI: init_jni_platform_state — переносим {} байт из pending в PlatformState",
            pending.len()
        );
        state.set_saved_state(pending.clone());
    }

    // Идемпотентная установка: если уже был — обновляем
    if GLOBAL_PLATFORM_STATE.get().is_some() {
        log::info!("JNI: init_jni_platform_state — обновляем существующий PlatformState");
        // Переносим saved_state_buffer из старого в новый если нужно
        if let Some(old_bytes) = GLOBAL_PLATFORM_STATE
            .get()
            .and_then(|s| s.take_saved_state())
        {
            state.set_saved_state(old_bytes);
        }
    }

    // OnceLock::set паникует при повторном вызове, поэтому используем трюк:
    // если уже установлен — игнорируем (состояние уже обновлено через get() выше)
    let _ = GLOBAL_PLATFORM_STATE.set(state);
    log::info!("JNI: GLOBAL_PLATFORM_STATE инициализирован");
}

/// Получить сериализованные данные из PlatformState и вернуть как jbyteArray.
///
/// Вызывается из Kotlin: `nativeGetSavedState(): ByteArray?`
///
/// Если буфер пуст — возвращает null.
#[no_mangle]
pub extern "system" fn Java_com_example_egui_1android_EguiActivity_nativeGetSavedState(
    mut env: JNIEnv,
    _class: jni::objects::JClass,
) -> jbyteArray {
    let state = match GLOBAL_PLATFORM_STATE.get() {
        Some(s) => s,
        None => {
            log::error!("JNI: nativeGetSavedState — GLOBAL_PLATFORM_STATE не инициализирован");
            return std::ptr::null_mut();
        }
    };

    let bytes = match state.take_saved_state() {
        Some(b) => b,
        None => {
            log::info!("JNI: nativeGetSavedState — буфер пуст, возвращаем null");
            return std::ptr::null_mut();
        }
    };

    log::info!(
        "JNI: nativeGetSavedState — передаём {} байт в Kotlin",
        bytes.len()
    );

    match env.new_byte_array(bytes.len() as i32) {
        Ok(jarr) => {
            let bytes_i8: &[i8] = unsafe { std::mem::transmute(bytes.as_slice()) };
            if let Err(e) = env.set_byte_array_region(&jarr, 0, bytes_i8) {
                log::error!("JNI: nativeGetSavedState — ошибка копирования: {}", e);
                return std::ptr::null_mut();
            }
            jarr.into_raw()
        }
        Err(e) => {
            log::error!(
                "JNI: nativeGetSavedState — ошибка создания byteArray: {}",
                e
            );
            std::ptr::null_mut()
        }
    }
}

/// Принять сериализованные данные из Android Bundle и сохранить в PlatformState.
///
/// Вызывается из Kotlin: `nativeSetSavedState(bytes: ByteArray?)`
///
/// Если `GLOBAL_PLATFORM_STATE` ещё не инициализирован — сохраняет
/// данные в `PENDING_SAVED_STATE`. Они будут перенесены в PlatformState
/// при вызове `init_jni_platform_state()`.
///
/// Если bytes == null — пропускаем (первый запуск без saved state).
#[no_mangle]
pub extern "system" fn Java_com_example_egui_1android_EguiActivity_nativeSetSavedState(
    mut env: JNIEnv,
    _class: jni::objects::JClass,
    byte_array: jni::objects::JByteArray,
) {
    // Проверяем, что byte_array не null
    if byte_array.is_null() {
        log::info!("JNI: nativeSetSavedState — byteArray == null, пропускаем (первый запуск)");
        return;
    }

    let len = match env.get_array_length(&byte_array) {
        Ok(l) => l,
        Err(e) => {
            log::error!("JNI: nativeSetSavedState — ошибка получения длины: {}", e);
            return;
        }
    };

    if len == 0 {
        log::info!("JNI: nativeSetSavedState — byteArray пуст, пропускаем");
        return;
    }

    // Копируем данные из Java byte[]
    let len = len as usize;
    let mut bytes_i8: Vec<i8> = vec![0i8; len];
    if let Err(e) = env.get_byte_array_region(&byte_array, 0, &mut bytes_i8) {
        log::error!("JNI: nativeSetSavedState — ошибка чтения byteArray: {}", e);
        return;
    }
    let bytes: Vec<u8> = unsafe { std::mem::transmute(bytes_i8) };

    log::info!(
        "JNI: nativeSetSavedState — получено {} байт из Bundle",
        bytes.len()
    );

    // Если PlatformState уже инициализирован — сохраняем напрямую
    if let Some(state) = GLOBAL_PLATFORM_STATE.get() {
        state.set_saved_state(bytes);
        return;
    }

    // PlatformState ещё не готов — сохраняем в pending
    log::info!(
        "JNI: nativeSetSavedState — GLOBAL_PLATFORM_STATE не инициализирован, сохраняем в pending"
    );
    if PENDING_SAVED_STATE.set(bytes).is_err() {
        log::error!("JNI: nativeSetSavedState — PENDING_SAVED_STATE уже заполнен");
    }
}
