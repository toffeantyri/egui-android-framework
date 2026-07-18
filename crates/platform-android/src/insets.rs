//! Определение системных отступов (insets) для Android-платформы.
//!
//! Архитектурные требования:
//! - Работает в контексте NativeActivity.
//! - Не трогает UI/Runtime/Reducer/Store.
//! - Источник истины: WindowInsets через JNI — хранятся в PlatformState.
//! - Fallback: content_rect + clamped insets.
//! - pp (pixels-per-point) считается из density_dpi.
//!
//! Глобальные статики отсутствуют. Insets хранятся в PlatformState внутри backend.

#![cfg(target_os = "android")]

use std::sync::Once;

static LOG_ONCE: Once = Once::new();

/// Insets в точках (points = пиксели / pp).
#[derive(Debug, Clone, Copy, Default)]
pub struct InsetsPt {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

// ─── JNI вызов через WindowInsets.Type.systemBars() ───────────────────────────

/// Получить системные insets через JNI.
///
/// Вызов Java API:
/// ```java
/// View view = activity.getWindow().getDecorView();
/// WindowInsets rootInsets = view.getRootWindowInsets();
/// android.graphics.Insets systemBars = rootInsets.getInsets(WindowInsets.Type.systemBars());
/// // systemBars.left, .top, .right, .bottom
/// ```
///
/// Возвращает `Some(crate::platform_state::InsetsPx)` если JNI успешен.
/// Вызов должен происходить на Java main thread.
pub fn get_system_insets_jni(
    vm_ptr: *mut std::ffi::c_void,
    activity_ptr: *mut std::ffi::c_void,
) -> Option<crate::platform_state::InsetsPx> {
    use jni::objects::{JObject, JValue};
    use jni::sys::jobject;

    if vm_ptr.is_null() || activity_ptr.is_null() {
        log::warn!("get_system_insets_jni: vm_ptr или activity_ptr null");
        return None;
    }

    unsafe {
        let jvm = jni::JavaVM::from_raw(vm_ptr as *mut jni::sys::JavaVM).ok()?;
        let mut env = jvm.attach_current_thread().ok()?;

        let activity = JObject::from_raw(activity_ptr as jobject);

        // window = activity.getWindow()
        let window = env
            .call_method(&activity, "getWindow", "()Landroid/view/Window;", &[])
            .ok()?
            .l()
            .ok()?;

        // decorView = window.getDecorView()
        let decor_view = env
            .call_method(&window, "getDecorView", "()Landroid/view/View;", &[])
            .ok()?
            .l()
            .ok()?;

        // rootInsets = decorView.getRootWindowInsets()
        let root_insets = env
            .call_method(
                &decor_view,
                "getRootWindowInsets",
                "()Landroid/view/WindowInsets;",
                &[],
            )
            .ok()?
            .l()
            .ok()?;

        // mask = WindowInsets.Type.systemBars()
        let type_class = env.find_class("android/view/WindowInsets$Type").ok()?;

        let system_bars_mask = env
            .call_static_method(&type_class, "systemBars", "()I", &[])
            .ok()?
            .i()
            .ok()?;

        // insetsObj = rootInsets.getInsets(mask) → android.graphics.Insets
        let insets_obj = env
            .call_method(
                &root_insets,
                "getInsets",
                "(I)Landroid/graphics/Insets;",
                &[JValue::Int(system_bars_mask)],
            )
            .ok()?
            .l()
            .ok()?;

        // Read fields: left, top, right, bottom
        let left = env.get_field(&insets_obj, "left", "I").ok()?.i().ok()?;
        let top = env.get_field(&insets_obj, "top", "I").ok()?.i().ok()?;
        let right = env.get_field(&insets_obj, "right", "I").ok()?.i().ok()?;
        let bottom = env.get_field(&insets_obj, "bottom", "I").ok()?.i().ok()?;

        let insets = crate::platform_state::InsetsPx {
            left,
            top,
            right,
            bottom,
        };

        LOG_ONCE.call_once(|| {
            log::info!(
                "JNI insets (WindowInsets.Type.systemBars): left={}, top={}, right={}, bottom={}",
                insets.left,
                insets.top,
                insets.right,
                insets.bottom
            );
        });

        Some(insets)
    }
}

// ─── Density (pixels per point) ──────────────────────────────────────────────

/// Получить pixels-per-point: pp = density_dpi / 160.
///
/// Если density недоступен — fallback на эвристику (размер экрана).
pub fn get_pp(config: &android_activity::ConfigurationRef, width: i32, height: i32) -> f32 {
    if let Some(density_dpi) = config.density() {
        if density_dpi > 0 {
            let pp = density_dpi as f32 / 160.0;
            LOG_ONCE.call_once(|| {
                log::info!(
                    "pp: system densityDpi={}, pixels_per_point={:.2}",
                    density_dpi,
                    pp
                );
            });
            return pp;
        }
    }

    // Fallback
    let w = width.max(height) as f32;
    let pp = (w / 450.0).max(1.5).min(4.0);
    LOG_ONCE.call_once(|| {
        log::info!(
            "pp: fallback эвристика для {}x{} → pp={:.2}",
            width,
            height,
            pp
        );
    });
    pp
}
