//! Управление системными барами Android: прозрачность, blur, цвета.
//!
//! Вызовы JNI на главном Java-потоке для настройки Window.
//! Используется в `run.rs` при инициализации окна (InitWindow).
//!
//! # Порядок проверки прозрачности
//!
//! 1. ANativeWindow_setBuffersGeometry с WINDOW_FORMAT_RGBA_8888
//! 2. EGL конфиг с EGL_ALPHA_SIZE = 8
//! 3. Window.setStatusBarColor(Color.TRANSPARENT) / setNavigationBarColor(Color.TRANSPARENT)
//! 4. DecorView.setSystemUiVisibility с флагами LAYOUT_*
//! 5. glClearColor(0, 0, 0, 0) перед каждым кадром
//! 6. egui_glow::Painter не перетирает прозрачность

#![cfg(target_os = "android")]

use jni::objects::JObject;
use jni::sys::jobject;
use std::sync::atomic::{AtomicPtr, Ordering};

/// Глобальные указатели на JavaVM и Activity (устанавливаются один раз при init).
static GLOBAL_VM: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(std::ptr::null_mut());
static GLOBAL_ACTIVITY: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(std::ptr::null_mut());

/// Сохранить указатели на JVM и Activity для последующего вызова apply_system_bars.
pub fn init_global_pointers(vm: *mut std::ffi::c_void, activity: *mut std::ffi::c_void) {
    GLOBAL_VM.store(vm, Ordering::Relaxed);
    GLOBAL_ACTIVITY.store(activity, Ordering::Relaxed);
}

/// Применить цвета системных баров для текущей темы, используя глобальные указатели.
/// Безопасно вызвать из любого потока (JNI attach делается внутри).
pub fn apply_system_bars_for_theme(theme: SystemTheme) {
    let vm = GLOBAL_VM.load(Ordering::Relaxed);
    let activity = GLOBAL_ACTIVITY.load(Ordering::Relaxed);
    if vm.is_null() || activity.is_null() {
        log::warn!("apply_system_bars_for_theme: глобальные указатели не инициализированы");
        return;
    }
    apply_system_bars_color_jni(vm, activity, theme);
}

/// Настроить системные бары: прозрачный статус-бар и навбар.
///
/// Логирует каждый шаг для диагностики.
pub fn set_transparent_system_bars(
    vm_ptr: *mut std::ffi::c_void,
    activity_ptr: *mut std::ffi::c_void,
) {
    log::info!("set_transparent_system_bars: начало");

    if vm_ptr.is_null() {
        log::error!("set_transparent_system_bars: vm_ptr is null");
        return;
    }
    if activity_ptr.is_null() {
        log::error!("set_transparent_system_bars: activity_ptr is null");
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

        let activity = JObject::from_raw(activity_ptr as jobject);

        // --- Шаг 1: getWindow ---
        log::info!("set_transparent_system_bars: вызываем activity.getWindow()");
        let window = match env
            .call_method(&activity, "getWindow", "()Landroid/view/Window;", &[])
            .ok()
            .and_then(|v| v.l().ok())
        {
            Some(w) => {
                log::info!("set_transparent_system_bars: getWindow OK");
                w
            }
            None => {
                log::error!("set_transparent_system_bars: getWindow вернул null");
                return;
            }
        };

        // --- Шаг 2: setStatusBarColor(Color.TRANSPARENT) ---
        log::info!("set_transparent_system_bars: вызываем setStatusBarColor(0)");
        match env.call_method(
            &window,
            "setStatusBarColor",
            "(I)V",
            &[jni::objects::JValue::Int(0)],
        ) {
            Ok(_) => log::info!("set_transparent_system_bars: setStatusBarColor OK"),
            Err(e) => log::error!(
                "set_transparent_system_bars: setStatusBarColor ошибка: {:?}",
                e
            ),
        }

        // --- Шаг 3: setNavigationBarColor(Color.TRANSPARENT) ---
        log::info!("set_transparent_system_bars: вызываем setNavigationBarColor(0)");
        match env.call_method(
            &window,
            "setNavigationBarColor",
            "(I)V",
            &[jni::objects::JValue::Int(0)],
        ) {
            Ok(_) => log::info!("set_transparent_system_bars: setNavigationBarColor OK"),
            Err(e) => log::error!(
                "set_transparent_system_bars: setNavigationBarColor ошибка: {:?}",
                e
            ),
        }

        // --- Шаг 4: getDecorView ---
        log::info!("set_transparent_system_bars: вызываем window.getDecorView()");
        let decor_view = match env
            .call_method(&window, "getDecorView", "()Landroid/view/View;", &[])
            .ok()
            .and_then(|v| v.l().ok())
        {
            Some(dv) => {
                log::info!("set_transparent_system_bars: getDecorView OK");
                dv
            }
            None => {
                log::error!("set_transparent_system_bars: getDecorView вернул null");
                return;
            }
        };

        // --- Шаг 5: setSystemUiVisibility с флагами LAYOUT_* ---
        // SYSTEM_UI_FLAG_LAYOUT_STABLE = 0x0100
        // SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN = 0x0400
        // SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION = 0x0200
        let flags = 0x0100 | 0x0400 | 0x0200;
        log::info!(
            "set_transparent_system_bars: вызываем setSystemUiVisibility(flags=0x{:x})",
            flags
        );
        match env.call_method(
            &decor_view,
            "setSystemUiVisibility",
            "(I)V",
            &[jni::objects::JValue::Int(flags)],
        ) {
            Ok(_) => log::info!("set_transparent_system_bars: setSystemUiVisibility OK"),
            Err(e) => log::error!(
                "set_transparent_system_bars: setSystemUiVisibility ошибка: {:?}",
                e
            ),
        }

        // --- Шаг 6: убираем фон Window (если Android рисует свой фон под барами) ---
        // window.setBackgroundDrawable(null)
        log::info!("set_transparent_system_bars: вызываем setBackgroundDrawable(null)");
        match env.call_method(
            &window,
            "setBackgroundDrawable",
            "(Landroid/graphics/drawable/Drawable;)V",
            &[jni::objects::JValue::Object(&jni::objects::JObject::null())],
        ) {
            Ok(_) => log::info!("set_transparent_system_bars: setBackgroundDrawable OK"),
            Err(e) => log::error!(
                "set_transparent_system_bars: setBackgroundDrawable ошибка: {:?}",
                e
            ),
        }

        // --- Шаг 7: проверяем, какой цвет установился (опционально) ---
        // Пробуем прочитать statusBarColor, чтобы убедиться что он стал 0
        match env.call_method(&window, "getStatusBarColor", "()I", &[]) {
            Ok(val) => {
                let color = val.i().unwrap_or(-1);
                log::info!(
                    "set_transparent_system_bars: getStatusBarColor после установки = 0x{:x}",
                    color
                );
            }
            Err(e) => log::error!(
                "set_transparent_system_bars: getStatusBarColor ошибка: {:?}",
                e
            ),
        }

        match env.call_method(&window, "getNavigationBarColor", "()I", &[]) {
            Ok(val) => {
                let color = val.i().unwrap_or(-1);
                log::info!(
                    "set_transparent_system_bars: getNavigationBarColor после установки = 0x{:x}",
                    color
                );
            }
            Err(e) => log::error!(
                "set_transparent_system_bars: getNavigationBarColor ошибка: {:?}",
                e
            ),
        }

        log::info!("set_transparent_system_bars: завершено");
    }
}

/// Установить blur для системных баров (Android 12+).
pub fn set_blur_for_system_bars(
    vm_ptr: *mut std::ffi::c_void,
    activity_ptr: *mut std::ffi::c_void,
    blur_radius: i32,
) {
    log::info!("set_blur_for_system_bars: начало, radius={}", blur_radius);

    if vm_ptr.is_null() || activity_ptr.is_null() {
        log::warn!("set_blur_for_system_bars: vm_ptr или activity_ptr null");
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

        let activity = JObject::from_raw(activity_ptr as jobject);

        let window = match env
            .call_method(&activity, "getWindow", "()Landroid/view/Window;", &[])
            .ok()
            .and_then(|v| v.l().ok())
        {
            Some(w) => w,
            None => {
                log::warn!("set_blur_for_system_bars: getWindow вернул null");
                return;
            }
        };

        log::info!(
            "set_blur_for_system_bars: вызываем setBackgroundBlurRadius({})",
            blur_radius
        );
        match env.call_method(
            &window,
            "setBackgroundBlurRadius",
            "(I)V",
            &[jni::objects::JValue::Int(blur_radius)],
        ) {
            Ok(_) => log::info!("set_blur_for_system_bars: setBackgroundBlurRadius OK"),
            Err(e) => log::error!(
                "set_blur_for_system_bars: setBackgroundBlurRadius ошибка: {:?}",
                e
            ),
        }

        log::info!("set_blur_for_system_bars: завершено");
    }
}

// ─── Определение системной темы ────────────────────────────────────────────────

use egui_android_core::SystemTheme;

/// Цвета для системных баров (status bar, navigation bar).
///
/// Приложение может установить свои цвета через `set_system_bars_colors()`.
#[derive(Clone, Copy, Debug)]
pub struct SystemBarsColors {
    /// Цвет для светлой темы (например, 0xFF_FF_FF = белый).
    pub light: u32,
    /// Цвет для тёмной темы (например, 0x00_00_00 = чёрный).
    pub dark: u32,
}

static SYSTEM_BARS_COLORS: std::sync::OnceLock<SystemBarsColors> = std::sync::OnceLock::new();

/// Установить цвета системных баров для светлой и тёмной темы.
pub fn set_system_bars_colors(colors: SystemBarsColors) {
    let _ = SYSTEM_BARS_COLORS.set(colors);
}

fn get_system_bars_colors() -> SystemBarsColors {
    SYSTEM_BARS_COLORS
        .get()
        .copied()
        .unwrap_or(SystemBarsColors {
            light: 0x00_00_00, // чёрный (по умолчанию)
            dark: 0x00_00_00,  // чёрный
        })
}

/// Определить системную тему Android через JNI.
///
/// Java-эквивалент:
/// ```java
/// int uiMode = activity.getResources().getConfiguration().uiMode;
/// int nightMode = uiMode & Configuration.UI_MODE_NIGHT_MASK;
/// if (nightMode == Configuration.UI_MODE_NIGHT_YES) return DARK;
/// else return LIGHT;
/// ```
pub fn detect_system_theme_jni(
    vm_ptr: *mut std::ffi::c_void,
    activity_ptr: *mut std::ffi::c_void,
) -> Option<SystemTheme> {
    if vm_ptr.is_null() || activity_ptr.is_null() {
        log::warn!("detect_system_theme_jni: vm_ptr или activity_ptr null");
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
            .ok()
            .and_then(|v| v.l().ok())
        {
            Some(r) => r,
            None => {
                log::warn!("detect_system_theme_jni: getResources вернул null");
                return None;
            }
        };

        // configuration = resources.getConfiguration()
        let configuration = match env
            .call_method(
                &resources,
                "getConfiguration",
                "()Landroid/content/res/Configuration;",
                &[],
            )
            .ok()
            .and_then(|v| v.l().ok())
        {
            Some(c) => c,
            None => {
                log::warn!("detect_system_theme_jni: getConfiguration вернул null");
                return None;
            }
        };

        // uiMode = configuration.uiMode
        let ui_mode = match env
            .get_field(&configuration, "uiMode", "I")
            .ok()
            .and_then(|v| v.i().ok())
        {
            Some(m) => m,
            None => {
                log::warn!("detect_system_theme_jni: uiMode поле не найдено");
                return None;
            }
        };

        // UI_MODE_NIGHT_MASK = 0x30
        // UI_MODE_NIGHT_YES = 0x20
        let night_mode = ui_mode & 0x30;
        let theme = if night_mode == 0x20 {
            SystemTheme::Dark
        } else {
            SystemTheme::Light
        };

        log::info!(
            "detect_system_theme_jni: uiMode={:#x}, night_mode={:#x}, theme={:?}",
            ui_mode,
            night_mode,
            theme
        );

        Some(theme)
    }
}

/// Применить цвет системных баров в соответствии с темой.
///
/// Вызывается после определения темы или после смены override.
pub fn apply_system_bars_color_jni(
    vm_ptr: *mut std::ffi::c_void,
    activity_ptr: *mut std::ffi::c_void,
    theme: SystemTheme,
) {
    if vm_ptr.is_null() || activity_ptr.is_null() {
        log::warn!("apply_system_bars_color_jni: vm_ptr или activity_ptr null");
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
            .ok()
            .and_then(|v| v.l().ok())
        {
            Some(w) => w,
            None => {
                log::warn!("apply_system_bars_color_jni: getWindow вернул null");
                return;
            }
        };

        let colors = get_system_bars_colors();
        let color = match theme {
            SystemTheme::Light => colors.light as i32,
            SystemTheme::Dark => colors.dark as i32,
        };

        // window.setStatusBarColor(color)
        let _ = env.call_method(
            &window,
            "setStatusBarColor",
            "(I)V",
            &[jni::objects::JValue::Int(color)],
        );

        // window.setNavigationBarColor(color)
        let _ = env.call_method(
            &window,
            "setNavigationBarColor",
            "(I)V",
            &[jni::objects::JValue::Int(color)],
        );

        // ─── Переключаем цвет иконок (светлые/тёмные) ─────────────────
        // Для светлой темы — тёмные иконки (APPEARANCE_LIGHT_STATUS_BARS),
        // для тёмной — светлые (без флага).
        //
        // Android 11+ (API 30): WindowInsetsController.setSystemBarsAppearance
        // Android 6-10 (API 23-29): View.setSystemUiVisibility(SYSTEM_UI_FLAG_LIGHT_STATUS_BAR)

        let decor_view = match env
            .call_method(&window, "getDecorView", "()Landroid/view/View;", &[])
            .ok()
            .and_then(|v| v.l().ok())
        {
            Some(dv) => dv,
            None => {
                log::warn!("apply_system_bars_color_jni: getDecorView вернул null");
                return;
            }
        };

        // Проверяем API level: пытаемся найти WindowInsetsController (API 30+)
        let insets_controller = env.call_method(
            &window,
            "getInsetsController",
            "()Landroid/view/WindowInsetsController;",
            &[],
        );

        match insets_controller {
            Ok(val) => {
                // API 30+ — используем WindowInsetsController
                if let Ok(controller) = val.l() {
                    if !controller.is_null() {
                        // APPEARANCE_LIGHT_STATUS_BARS    = 8  (бит 3)
                        // APPEARANCE_LIGHT_NAVIGATION_BARS = 16 (бит 4)
                        let mask = 8i32 | 16i32; // статус-бар + навбар
                        let appearance = match theme {
                            SystemTheme::Light => mask,
                            SystemTheme::Dark => 0i32,
                        };
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
                            "apply_system_bars_color_jni: setSystemBarsAppearance(appearance={}, mask={}), theme={:?}",
                            appearance, mask, theme
                        );
                    }
                }
            }
            Err(_) => {
                // API 23-29 — фолбэк на SYSTEM_UI_FLAG_LIGHT_STATUS_BAR
                let flag_light_status_bar = 0x00002000i32; // SYSTEM_UI_FLAG_LIGHT_STATUS_BAR
                let flag_light_nav_bar = 0x00000010i32; // SYSTEM_UI_FLAG_LIGHT_NAVIGATION_BAR

                // Получаем текущие флаги
                let current_flags = env
                    .call_method(&decor_view, "getSystemUiVisibility", "()I", &[])
                    .ok()
                    .and_then(|v| Some(v.i().ok()?))
                    .unwrap_or(0);

                let new_flags = match theme {
                    SystemTheme::Light => {
                        current_flags | flag_light_status_bar | flag_light_nav_bar
                    }
                    SystemTheme::Dark => {
                        current_flags & !flag_light_status_bar & !flag_light_nav_bar
                    }
                };

                let _ = env.call_method(
                    &decor_view,
                    "setSystemUiVisibility",
                    "(I)V",
                    &[jni::objects::JValue::Int(new_flags)],
                );
                log::info!(
                    "apply_system_bars_color_jni: setSystemUiVisibility(0x{:x}), theme={:?}",
                    new_flags,
                    theme
                );
            }
        }

        log::info!(
            "apply_system_bars_color_jni: theme={:?}, color=0x{:08x}",
            theme,
            color as u32
        );
    }
}
