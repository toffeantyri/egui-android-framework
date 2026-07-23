//! Управление системными барами Android: прозрачность, blur, цвета.
//!
//! Вызовы JNI на главном Java-потоке для настройки Window.
//! Все функции принимают vm_ptr и activity_ptr явно — глобальные статики отсутствуют.
//!
//! # Порядок проверки прозрачности
//!
//! 1. ANativeWindow_setBuffersGeometry с WINDOW_FORMAT_RGBA_8888
//! 2. EGL конфиг с EGL_ALPHA_SIZE = 8
//! 3. Window.setStatusBarColor(Color.TRANSPARENT) / setNavigationBarColor(Color.TRANSPARENT)
//! 4. WindowInsetsController.setSystemBarsAppearance() для цвета иконок
//! 5. glClearColor(0, 0, 0, 0) перед каждым кадром
//! 6. egui_glow::Painter не перетирает прозрачность
//!
//! setSystemUiVisibility и setBackgroundDrawable удалены — deprecated в Android 16.

#![cfg(target_os = "android")]

use crate::platform_state::PlatformState;
use egui_android_platform::SystemTheme;

pub use self::inner::*;

/// Применить цвета системных баров для текущей темы через PlatformState в backend.
///
/// Вызывается из platform-android при инициализации или при смене темы.
pub fn apply_system_bars_for_platform_state(state: &PlatformState) {
    let vm = state.vm_ptr();
    let activity = state.activity_ptr();
    if !vm.is_null() && !activity.is_null() {
        let theme = state.current_theme();
        inner::apply_system_bars_color_jni(vm, activity, theme);
    }
}

mod inner {
    use egui_android_platform::SystemTheme;

    /// Настроить системные бары: прозрачный статус-бар и навбар.
    ///
    /// Логирует каждый шаг для диагностики.
    pub fn set_transparent_system_bars(
        vm_ptr: *mut std::ffi::c_void,
        activity_ptr: *mut std::ffi::c_void,
    ) {
        log::info!("set_transparent_system_bars: начало");

        if vm_ptr.is_null() || activity_ptr.is_null() {
            log::error!("set_transparent_system_bars: null pointer");
            return;
        }

        unsafe {
            let jvm = match jni::JavaVM::from_raw(vm_ptr as *mut jni::sys::JavaVM) {
                Ok(jvm) => jvm,
                Err(e) => {
                    log::error!("set_transparent_system_bars: JavaVM::from_raw: {:?}", e);
                    return;
                }
            };
            let mut env = match jvm.attach_current_thread() {
                Ok(env) => env,
                Err(e) => {
                    log::error!(
                        "set_transparent_system_bars: attach_current_thread: {:?}",
                        e
                    );
                    return;
                }
            };

            let activity = jni::objects::JObject::from_raw(activity_ptr as jni::sys::jobject);

            // --- Шаг 1: getWindow ---
            let window = match env
                .call_method(&activity, "getWindow", "()Landroid/view/Window;", &[])
                .and_then(|v| v.l())
            {
                Ok(w) => w,
                Err(e) => {
                    log::error!("set_transparent_system_bars: getWindow: {:?}", e);
                    return;
                }
            };

            // --- Шаг 2: setStatusBarColor(0) ---
            match env.call_method(
                &window,
                "setStatusBarColor",
                "(I)V",
                &[jni::objects::JValue::Int(0)],
            ) {
                Ok(_) => log::info!("set_transparent_system_bars: StatusBar → TRANSPARENT"),
                Err(e) => {
                    log::warn!("setStatusBarColor: {:?}", e);
                }
            }

            // --- Шаг 3: setNavigationBarColor(0) ---
            match env.call_method(
                &window,
                "setNavigationBarColor",
                "(I)V",
                &[jni::objects::JValue::Int(0)],
            ) {
                Ok(_) => log::info!("set_transparent_system_bars: NavigationBar → TRANSPARENT"),
                Err(e) => {
                    log::warn!("setNavigationBarColor: {:?}", e);
                }
            }
        }

        log::info!("set_transparent_system_bars: завершено");
    }

    /// Включить blur под системными барами (Android 12+).
    pub fn set_blur_for_system_bars(
        vm_ptr: *mut std::ffi::c_void,
        activity_ptr: *mut std::ffi::c_void,
        blur_radius: i32,
    ) {
        if vm_ptr.is_null() || activity_ptr.is_null() {
            log::warn!("set_blur_for_system_bars: null pointer");
            return;
        }

        unsafe {
            let jvm = match jni::JavaVM::from_raw(vm_ptr as *mut jni::sys::JavaVM) {
                Ok(jvm) => jvm,
                Err(e) => {
                    log::warn!("set_blur_for_system_bars: JavaVM::from_raw: {:?}", e);
                    return;
                }
            };
            let mut env = match jvm.attach_current_thread() {
                Ok(env) => env,
                Err(e) => {
                    log::warn!("set_blur_for_system_bars: attach_current_thread: {:?}", e);
                    return;
                }
            };

            let activity = jni::objects::JObject::from_raw(activity_ptr as jni::sys::jobject);

            let window = match env
                .call_method(&activity, "getWindow", "()Landroid/view/Window;", &[])
                .and_then(|v| v.l())
            {
                Ok(w) => w,
                Err(e) => {
                    log::warn!("set_blur_for_system_bars: getWindow: {:?}", e);
                    return;
                }
            };

            // setBackgroundBlurRadius(int) — Android 12+ (API 31+)
            match env.call_method(
                &window,
                "setBackgroundBlurRadius",
                "(I)V",
                &[jni::objects::JValue::Int(blur_radius)],
            ) {
                Ok(_) => {
                    log::info!("set_blur_for_system_bars: blur radius={}", blur_radius);
                }
                Err(e) => {
                    log::warn!("setBackgroundBlurRadius: {:?}", e);
                }
            }
        }
    }

    /// Определить системную тему (Light/Dark) через JNI.
    ///
    /// Использует `Resources.getConfiguration().uiMode & UI_MODE_NIGHT_MASK`.
    pub fn detect_system_theme_jni(
        vm_ptr: *mut std::ffi::c_void,
        activity_ptr: *mut std::ffi::c_void,
    ) -> Option<SystemTheme> {
        if vm_ptr.is_null() || activity_ptr.is_null() {
            log::warn!("detect_system_theme_jni: null pointer");
            return None;
        }

        unsafe {
            let jvm = match jni::JavaVM::from_raw(vm_ptr as *mut jni::sys::JavaVM) {
                Ok(jvm) => jvm,
                Err(e) => {
                    log::warn!("detect_system_theme_jni: JavaVM::from_raw: {:?}", e);
                    return None;
                }
            };
            let mut env = match jvm.attach_current_thread() {
                Ok(env) => env,
                Err(e) => {
                    log::warn!("detect_system_theme_jni: attach_current_thread: {:?}", e);
                    return None;
                }
            };

            let activity = jni::objects::JObject::from_raw(activity_ptr as jni::sys::jobject);

            // resources = activity.getResources()
            let resources = match env
                .call_method(
                    &activity,
                    "getResources",
                    "()Landroid/content/res/Resources;",
                    &[],
                )
                .and_then(|v| v.l())
            {
                Ok(r) => r,
                Err(e) => {
                    log::warn!("detect_system_theme: getResources: {:?}", e);
                    return None;
                }
            };

            // config = resources.getConfiguration()
            let config = match env
                .call_method(
                    &resources,
                    "getConfiguration",
                    "()Landroid/content/res/Configuration;",
                    &[],
                )
                .and_then(|v| v.l())
            {
                Ok(c) => c,
                Err(e) => {
                    log::warn!("detect_system_theme: getConfiguration: {:?}", e);
                    return None;
                }
            };

            // uiMode = config.uiMode (int)
            let ui_mode = match env.get_field(&config, "uiMode", "I").and_then(|v| v.i()) {
                Ok(m) => m,
                Err(e) => {
                    log::warn!("detect_system_theme: uiMode field: {:?}", e);
                    return None;
                }
            };

            const UI_MODE_NIGHT_MASK: i32 = 0x20;
            const UI_MODE_NIGHT_YES: i32 = 0x20;

            let theme = if ui_mode & UI_MODE_NIGHT_MASK == UI_MODE_NIGHT_YES {
                SystemTheme::Dark
            } else {
                SystemTheme::Light
            };

            log::info!(
                "detect_system_theme_jni: uiMode={:#x} → {:?}",
                ui_mode,
                theme
            );

            Some(theme)
        }
    }

    /// Применить цвета системных баров через JNI (статус-бар + навбар).
    pub fn apply_system_bars_color_jni(
        vm_ptr: *mut std::ffi::c_void,
        activity_ptr: *mut std::ffi::c_void,
        theme: SystemTheme,
    ) {
        if vm_ptr.is_null() || activity_ptr.is_null() {
            log::warn!("apply_system_bars_color_jni: null pointer");
            return;
        }

        unsafe {
            let jvm = match jni::JavaVM::from_raw(vm_ptr as *mut jni::sys::JavaVM) {
                Ok(jvm) => jvm,
                Err(e) => {
                    log::warn!("apply_system_bars_color_jni: JavaVM::from_raw: {:?}", e);
                    return;
                }
            };
            let mut env = match jvm.attach_current_thread() {
                Ok(env) => env,
                Err(e) => {
                    log::warn!(
                        "apply_system_bars_color_jni: attach_current_thread: {:?}",
                        e
                    );
                    return;
                }
            };

            let activity = jni::objects::JObject::from_raw(activity_ptr as jni::sys::jobject);

            // window = activity.getWindow()
            let window = match env
                .call_method(&activity, "getWindow", "()Landroid/view/Window;", &[])
                .and_then(|v| v.l())
            {
                Ok(w) => w,
                Err(e) => {
                    log::warn!("apply_system_bars_color_jni: getWindow: {:?}", e);
                    return;
                }
            };

            // Цвет баров: чёрный для светлой темы, белый для тёмной
            let color: i32 = match theme {
                SystemTheme::Light => 0x00_00_00, // чёрный статус-бар
                SystemTheme::Dark => 0xFF_FF_FF,  // белый статус-бар
            };

            // setStatusBarColor
            let _ = env.call_method(
                &window,
                "setStatusBarColor",
                "(I)V",
                &[jni::objects::JValue::Int(color)],
            );

            // setNavigationBarColor
            let _ = env.call_method(
                &window,
                "setNavigationBarColor",
                "(I)V",
                &[jni::objects::JValue::Int(color)],
            );

            // --- Управление цветом иконок системных баров ---
            // WindowInsetsController.setSystemBarsAppearance() — API 30+
            // APPEARANCE_LIGHT_STATUS_BARS = 0x0000_0008: тёмные иконки (для светлого фона)
            // APPEARANCE_LIGHT_NAVIGATION_BARS = 0x0000_0010: тёмные кнопки навбара
            // По умолчанию (0) — светлые иконки (для тёмного фона)
            let insets_controller = env
                .call_method(
                    &window,
                    "getInsetsController",
                    "()Landroid/view/WindowInsetsController;",
                    &[],
                )
                .and_then(|v| v.l());

            if let Ok(controller) = insets_controller {
                let appearance: i32 = match theme {
                    SystemTheme::Light => 0x0000_0008 | 0x0000_0010, // тёмные иконки
                    SystemTheme::Dark => 0, // светлые иконки (по умолчанию)
                };
                let mask: i32 = 0x0000_0008 | 0x0000_0010; // оба флага в маске
                let _ = env.call_method(
                    &controller,
                    "setSystemBarsAppearance",
                    "(II)V",
                    &[
                        jni::objects::JValue::Int(appearance),
                        jni::objects::JValue::Int(mask),
                    ],
                );
                log::info!(
                    "apply_system_bars_color_jni: setSystemBarsAppearance appearance={:#x} mask={:#x}",
                    appearance,
                    mask,
                );
            } else {
                log::warn!(
                    "apply_system_bars_color_jni: getInsetsController недоступен (API < 30?)"
                );
            }

            log::info!(
                "apply_system_bars_color_jni: theme={:?} color=#{:06x}",
                theme,
                color,
            );
        }
    }
}
