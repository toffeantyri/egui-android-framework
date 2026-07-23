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
//! # Именование JNI-функций
//!
//! JNI-функции используют формат `Java_<package>_<Class>_<method>`.
//! Package: `com.example.egui_android` (из AndroidManifest + EguiActivity).
//! Class: `EguiActivity`.
//!
//! Функции доступны через `extern "C"` и вызываются из Kotlin.

#![cfg(target_os = "android")]

use crate::platform_state::PlatformState;
use jni::objects::JByteArray;
use jni::sys::{jbyteArray, jobject};
use jni::JNIEnv;
use std::sync::OnceLock;

/// Глобальная ссылка на PlatformState для доступа из JNI-функций.
///
/// Инициализируется в `run_with_backend()` после создания backend'а.
/// JNI-функции вызываются на UI thread, блокировка Mutex внутри PlatformState
/// гарантирует безопасность.
static GLOBAL_PLATFORM_STATE: OnceLock<PlatformState> = OnceLock::new();

/// Зарегистрировать PlatformState для доступа из JNI.
///
/// Вызывается из `run_with_backend()` один раз при старте.
pub fn init_jni_platform_state(state: PlatformState) {
    GLOBAL_PLATFORM_STATE
        .set(state)
        .expect("GLOBAL_PLATFORM_STATE уже инициализирован");
    log::info!("JNI: GLOBAL_PLATFORM_STATE инициализирован");
}

/// Получить сериализованные данные из PlatformState и вернуть как jbyteArray.
///
/// Вызывается из Kotlin: `nativeGetSavedState(): ByteArray?`
///
/// Если буфер пуст — возвращает null.
///
/// # Безопасность
///
/// Вызывается на UI thread через JNI. Блокировка Mutex внутри PlatformState
/// минимальна (копирование Vec<u8>).
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

    // Создаём Java byte[] и копируем данные
    match env.new_byte_array(bytes.len() as i32) {
        Ok(jarr) => {
            // jni 0.21 требует &[i8], конвертируем u8 -> i8 через transmute
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
/// Если bytes == null — буфер не изменяется (первый запуск).
///
/// # Безопасность
///
/// Вызывается на UI thread через JNI. Блокировка Mutex внутри PlatformState
/// минимальна (сохранение Vec<u8>).
#[no_mangle]
pub extern "system" fn Java_com_example_egui_1android_EguiActivity_nativeSetSavedState(
    mut env: JNIEnv,
    _class: jni::objects::JClass,
    byte_array: jni::objects::JByteArray,
) {
    let state = match GLOBAL_PLATFORM_STATE.get() {
        Some(s) => s,
        None => {
            log::error!("JNI: nativeSetSavedState — GLOBAL_PLATFORM_STATE не инициализирован");
            return;
        }
    };

    // Проверяем, что byte_array не null (в Kotlin передаётся ByteArray?)
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
    // jni 0.21 требует &mut [i8], конвертируем u8 -> i8 через transmute
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

    state.set_saved_state(bytes);
}
