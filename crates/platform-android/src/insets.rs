//! Определение системных отступов (insets) для Android-платформы.
//!
//! Архитектурные требования:
//! - Работает в контексте NativeActivity.
//! - Не трогает UI/Runtime/Reducer/Store.
//! - Источник истины: WindowInsets через JNI.
//! - Fallback: content_rect + clamped insets.
//! - pp (pixels-per-point) считается из density_dpi.

#![cfg(target_os = "android")]

use jni::objects::{JObject, JValue};
use jni::sys::jobject;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Once;

static LOG_ONCE: Once = Once::new();

// ─── Структуры данных ────────────────────────────────────────────────────────

/// Insets в пикселях.
#[derive(Debug, Clone, Copy, Default)]
pub struct InsetsPx {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl InsetsPx {
    /// Все значения ≥ 0 (установлены через JNI).
    pub fn is_valid(&self) -> bool {
        self.left >= 0 && self.top >= 0 && self.right >= 0 && self.bottom >= 0
    }
}

/// Insets в точках (points = пиксели / pp).
#[derive(Debug, Clone, Copy, Default)]
pub struct InsetsPt {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

// ─── Глобальные insets в пикселях (заполняются из JNI) ──────────────────────

static INSET_LEFT: AtomicI32 = AtomicI32::new(-1);
static INSET_TOP: AtomicI32 = AtomicI32::new(-1);
static INSET_RIGHT: AtomicI32 = AtomicI32::new(-1);
static INSET_BOTTOM: AtomicI32 = AtomicI32::new(-1);

fn store_insets_px(insets: &InsetsPx) {
    INSET_LEFT.store(insets.left, Ordering::Relaxed);
    INSET_TOP.store(insets.top, Ordering::Relaxed);
    INSET_RIGHT.store(insets.right, Ordering::Relaxed);
    INSET_BOTTOM.store(insets.bottom, Ordering::Relaxed);
}

fn load_insets_px() -> InsetsPx {
    InsetsPx {
        left: INSET_LEFT.load(Ordering::Relaxed),
        top: INSET_TOP.load(Ordering::Relaxed),
        right: INSET_RIGHT.load(Ordering::Relaxed),
        bottom: INSET_BOTTOM.load(Ordering::Relaxed),
    }
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
/// Возвращает `Some(InsetsPx)` если JNI успешен.
/// Вызов должен происходить на Java main thread.
pub fn get_system_insets_jni(
    vm_ptr: *mut std::ffi::c_void,
    activity_ptr: *mut std::ffi::c_void,
) -> Option<InsetsPx> {
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

        let insets = InsetsPx {
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

/// Сохранить insets в глобальные переменные (вызывается из Java main thread).
pub fn store_insets(insets: &InsetsPx) {
    store_insets_px(insets);
    log::info!(
        "Insets сохранены: left={}, top={}, right={}, bottom={}",
        insets.left,
        insets.top,
        insets.right,
        insets.bottom
    );
}

// ─── Публичный API для платформенного слоя ────────────────────────────────────

/// Получить insets в точках (points) с приоритетом:
/// 1. JNI (WindowInsets.Type.systemBars)
/// 2. Fallback: content_rect с clamp (top ≤ 32dp, bottom ≤ 48dp, left/right ≤ 32dp)
///
/// density берётся из конфигурации: pp = density_dpi / 160.
pub fn get_insets_pt(
    config: &android_activity::ConfigurationRef,
    native_width: i32,
    native_height: i32,
    content_rect: &android_activity::Rect,
) -> InsetsPt {
    let pp = get_pp(config, native_width, native_height);
    let jni_insets = load_insets_px();

    if jni_insets.is_valid() {
        LOG_ONCE.call_once(|| {
            log::info!(
                "Insets (JNI): left={:.1}, top={:.1}, right={:.1}, bottom={:.1} pt (pp={:.2})",
                jni_insets.left as f32 / pp,
                jni_insets.top as f32 / pp,
                jni_insets.right as f32 / pp,
                jni_insets.bottom as f32 / pp,
                pp
            );
        });

        return InsetsPt {
            left: jni_insets.left as f32 / pp,
            top: jni_insets.top as f32 / pp,
            right: jni_insets.right as f32 / pp,
            bottom: jni_insets.bottom as f32 / pp,
        };
    }

    // Fallback: content_rect + clamp
    let top_px = content_rect.top;
    let bottom_px = native_height - content_rect.bottom;
    let left_px = content_rect.left;
    let right_px = native_width - content_rect.right;

    let mut top_pt = top_px as f32 / pp;
    let mut bottom_pt = bottom_px as f32 / pp;
    let mut left_pt = left_px as f32 / pp;
    let mut right_pt = right_px as f32 / pp;

    // Ограничения: статус-бар ~24-32dp, навбар/жесты ~48dp
    top_pt = top_pt.clamp(0.0, 32.0);
    bottom_pt = bottom_pt.clamp(0.0, 48.0);
    left_pt = left_pt.clamp(0.0, 32.0);
    right_pt = right_pt.clamp(0.0, 32.0);

    LOG_ONCE.call_once(|| {
        log::info!(
            "Insets (fallback content_rect): left={:.1}, top={:.1}, right={:.1}, bottom={:.1} pt \
             (raw px: left={}, top={}, right={}, bottom={}, pp={:.2})",
            left_pt,
            top_pt,
            right_pt,
            bottom_pt,
            left_px,
            top_px,
            right_px,
            bottom_px,
            pp
        );
    });

    InsetsPt {
        left: left_pt,
        top: top_pt,
        right: right_pt,
        bottom: bottom_pt,
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
