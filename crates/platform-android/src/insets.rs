//! Определение insets (status bar, navigation bar, display cutout) через JNI.
//!
//! Использует `View.getRootWindowInsets()` и `WindowInsets.getInsets(WindowInsets.Type.systemBars())`
//! для получения реальных системных отступов.
//!
//! JNI вызывается из главного потока через `AndroidApp::run_on_java_main_thread()`.
//! Insets сохраняются в глобальных atomic-переменных и читаются синхронно из цикла рендеринга.
//!
//! Fallback для Android < 23 или если JNI не сработал:
//! используем `content_rect` с ограничениями (top ≤ 32dp, bottom ≤ 48dp).

#![cfg(target_os = "android")]

use jni::objects::JObject;
use jni::sys::jobject;
use jni::JNIEnv;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Once;

static LOG_ONCE: Once = Once::new();

// ─── Глобальные insets в пикселях (заполняются из JNI) ──────────────────────

static INSET_LEFT: AtomicI32 = AtomicI32::new(-1);
static INSET_TOP: AtomicI32 = AtomicI32::new(-1);
static INSET_RIGHT: AtomicI32 = AtomicI32::new(-1);
static INSET_BOTTOM: AtomicI32 = AtomicI32::new(-1);

/// Структура с insets в пикселях.
#[derive(Debug, Clone, Copy, Default)]
pub struct InsetsPx {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl InsetsPx {
    /// Insets валидны (все ≥ 0).
    pub fn is_valid(&self) -> bool {
        self.top >= 0 && self.bottom >= 0 && self.left >= 0 && self.right >= 0
    }

    /// JNI сработал (top >= 0 означает, что store был вызван).
    pub fn is_jni_loaded(&self) -> bool {
        self.top >= 0
    }
}

// ─── JNI вызов: получение системных inset ────────────────────────────────────

/// Получить системные insets через JNI.
///
/// Вызов Java API:
/// ```java
/// View view = activity.getWindow().getDecorView();
/// WindowInsets insets = view.getRootWindowInsets();
/// if (insets != null) {
///     android.graphics.Insets systemBars = insets.getInsets(WindowInsets.Type.systemBars());
///     // systemBars.left, .top, .right, .bottom
/// }
/// ```
///
/// Возвращает `Some(InsetsPx)` в пикселях, если JNI вызов успешен.
pub fn get_system_insets_jni(
    vm_ptr: *mut std::ffi::c_void,
    activity_ptr: *mut std::ffi::c_void,
) -> Option<InsetsPx> {
    if vm_ptr.is_null() || activity_ptr.is_null() {
        log::warn!("get_system_insets_jni: vm_ptr или activity_ptr null");
        return None;
    }

    unsafe {
        let jvm = match jni::JavaVM::from_raw(vm_ptr as *mut jni::sys::JavaVM) {
            Ok(jvm) => jvm,
            Err(e) => {
                log::error!("get_system_insets_jni: JavaVM::from_raw не удался: {:?}", e);
                return None;
            }
        };
        let mut env = match jvm.attach_current_thread() {
            Ok(env) => env,
            Err(e) => {
                log::error!(
                    "get_system_insets_jni: attach_current_thread не удался: {:?}",
                    e
                );
                return None;
            }
        };

        let result = call_get_system_insets(&mut env, activity_ptr);

        // attach_current_thread возвращает Guard, который detach-ит при drop.
        // Это нормально — run_on_java_main_thread сам управляет прикреплением.

        match result {
            Ok(insets) => {
                LOG_ONCE.call_once(|| {
                    log::info!(
                        "JNI insets: left={}, top={}, right={}, bottom={}",
                        insets.left,
                        insets.top,
                        insets.right,
                        insets.bottom
                    );
                });
                Some(insets)
            }
            Err(e) => {
                log::error!("get_system_insets_jni: ошибка JNI: {:?}", e);
                None
            }
        }
    }
}

/// Внутренняя функция — делает JNI вызовы.
unsafe fn call_get_system_insets(
    env: &mut JNIEnv,
    activity_ptr: *mut std::ffi::c_void,
) -> Result<InsetsPx, String> {
    // 1. Получаем activity как JObject
    let activity = JObject::from_raw(activity_ptr as jobject);

    // 2. Получаем Window через getWindow()
    let window_obj = env
        .call_method(&activity, "getWindow", "()Landroid/view/Window;", &[])
        .map_err(|e| format!("getWindow() failed: {:?}", e))?
        .l()
        .map_err(|e| format!("getWindow() вернул не объект: {:?}", e))?;

    // 3. Получаем DecorView через getDecorView()
    let decor_view = env
        .call_method(&window_obj, "getDecorView", "()Landroid/view/View;", &[])
        .map_err(|e| format!("getDecorView() failed: {:?}", e))?
        .l()
        .map_err(|e| format!("getDecorView() вернул не объект: {:?}", e))?;

    // 4. Вызываем getRootWindowInsets() — доступно с API 23
    let root_insets_local = env
        .call_method(
            &decor_view,
            "getRootWindowInsets",
            "()Landroid/view/WindowInsets;",
            &[],
        )
        .map_err(|e| format!("getRootWindowInsets() failed: {:?}", e))?
        .l()
        .map_err(|e| format!("getRootWindowInsets() вернул не объект: {:?}", e))?;

    // Создаём глобальную ссылку, чтобы объект не был собран GC между вызовами
    let root_insets = env
        .new_global_ref(&root_insets_local)
        .map_err(|e| format!("new_global_ref failed: {:?}", e))?;

    // 5. Используем универсальный метод — getSystemWindowInsetTop/Bottom/Left/Right
    //    Работает на API 20+ без проблем с совместимостью
    let top = call_int_method(env, root_insets.as_obj(), "getSystemWindowInsetTop", "()I")?;
    let bottom = call_int_method(
        env,
        root_insets.as_obj(),
        "getSystemWindowInsetBottom",
        "()I",
    )?;
    let left = call_int_method(env, root_insets.as_obj(), "getSystemWindowInsetLeft", "()I")?;
    let right = call_int_method(
        env,
        root_insets.as_obj(),
        "getSystemWindowInsetRight",
        "()I",
    )?;

    // Дополнительно, если SDK >= 29 и есть cutout — можно получить display cutout insets
    // через WindowInsets.getInsets(Type.systemBars()). Для начала хватит systemWindow.

    Ok(InsetsPx {
        left,
        top,
        right,
        bottom,
    })
}

unsafe fn call_int_method(
    env: &mut JNIEnv,
    obj: &JObject,
    method: &str,
    sig: &str,
) -> Result<i32, String> {
    let val = env
        .call_method(obj, method, sig, &[])
        .map_err(|e| format!("{}() failed: {:?}", method, e))?
        .i()
        .map_err(|e| format!("{}() .i() failed: {:?}", method, e))?;

    Ok(val)
}

// ─── Хранение insets глобально ───────────────────────────────────────────────

/// Сохранить insets в глобальные переменные (вызывается из главного потока).
pub fn store_insets(insets: &InsetsPx) {
    INSET_LEFT.store(insets.left, Ordering::Relaxed);
    INSET_TOP.store(insets.top, Ordering::Relaxed);
    INSET_RIGHT.store(insets.right, Ordering::Relaxed);
    INSET_BOTTOM.store(insets.bottom, Ordering::Relaxed);

    log::info!(
        "Insets сохранены: left={}, top={}, right={}, bottom={}",
        insets.left,
        insets.top,
        insets.right,
        insets.bottom
    );
}

/// Прочитать insets из глобальных переменных.
pub fn load_insets() -> InsetsPx {
    InsetsPx {
        left: INSET_LEFT.load(Ordering::Relaxed),
        top: INSET_TOP.load(Ordering::Relaxed),
        right: INSET_RIGHT.load(Ordering::Relaxed),
        bottom: INSET_BOTTOM.load(Ordering::Relaxed),
    }
}

// ─── Публичный API ───────────────────────────────────────────────────────────

/// Получить инсеты в точках (points) с приоритетом:
/// 1. JNI insets (если загружены и валидны)
/// 2. Fallback через content_rect (с ограничениями top ≤ 32dp, bottom ≤ 48dp)
///
/// Возвращает (left, top, right, bottom) в точках.
pub fn get_insets_pt(
    native_width: i32,
    native_height: i32,
    content_rect: &android_activity::Rect,
    pp: f32,
) -> (f32, f32, f32, f32) {
    let jni_insets = load_insets();

    if jni_insets.is_jni_loaded() {
        // JNI сработал — используем реальные инсеты
        let left = jni_insets.left as f32 / pp;
        let top = jni_insets.top as f32 / pp;
        let right = jni_insets.right as f32 / pp;
        let bottom = jni_insets.bottom as f32 / pp;

        LOG_ONCE.call_once(|| {
            log::info!(
                "Insets (JNI): left={:.1}, top={:.1}, right={:.1}, bottom={:.1} pt (pp={})",
                left,
                top,
                right,
                bottom,
                pp
            );
        });

        (left, top, right, bottom)
    } else {
        // JNI не сработал — fallback на content_rect с ограничениями
        // Верхний inset не более 32dp, нижний — не более 48dp
        let max_top_dp = 32.0;
        let max_bottom_dp = 48.0;

        let top_raw = content_rect.top as f32 / pp;
        let top = top_raw.min(max_top_dp);

        let bottom_raw = if content_rect.bottom >= native_height {
            0.0
        } else {
            (native_height - content_rect.bottom) as f32 / pp
        };
        let bottom = bottom_raw.min(max_bottom_dp);

        let left = content_rect.left as f32 / pp;
        let right = if content_rect.right >= native_width {
            0.0
        } else {
            (native_width - content_rect.right) as f32 / pp
        };

        LOG_ONCE.call_once(|| {
            log::info!(
                "Insets (fallback): left={:.1}, top={:.1} (raw={:.1}, max={}), right={:.1}, bottom={:.1} (raw={:.1}, max={}) pt (pp={})",
                left, top, top_raw, max_top_dp, right, bottom, bottom_raw, max_bottom_dp, pp
            );
        });

        (left, top, right, bottom)
    }
}

/// Получить density (pixels per point) из конфигурации Android.
///
/// `config.density()` возвращает `densityDpi` устройства.
/// `pixels_per_point = density_dpi / 160`.
///
/// Если density недоступен — использует эвристику (fallback).
pub fn get_density(config: &android_activity::ConfigurationRef, width: i32, height: i32) -> f32 {
    // Пробуем получить реальную плотность из конфигурации Android
    if let Some(density_dpi) = config.density() {
        if density_dpi > 0 {
            let pp = density_dpi as f32 / 160.0;
            LOG_ONCE.call_once(|| {
                log::info!(
                    "density: system densityDpi={}, pixels_per_point={:.2}",
                    density_dpi,
                    pp
                );
            });
            return pp;
        }
    }

    // Fallback: эвристика на основе размера экрана
    let w = width.max(height) as f32;
    let pp = (w / 450.0).max(1.5).min(4.0);
    LOG_ONCE.call_once(|| {
        log::info!(
            "density: fallback эвристика для {}x{} → pp={:.2}",
            width,
            height,
            pp
        );
    });
    pp
}
