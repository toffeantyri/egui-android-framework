//! Обработка событий жизненного цикла Android.
//!
//! Вынесена из `run.rs` для разделения ответственности.
//!
//! Все функции generic по `A: Application`, так как `Application` не object-safe
//! (требует `Self: Sized`).
//!
//! # Архитектура
//!
//! `handle_lifecycle_event` — единая точка входа для всех lifecycle-событий.
//! Вызывается из главного цикла в `run.rs`.

#![cfg(target_os = "android")]

use crate::backend::AndroidBackend;
use crate::event::LifecycleEvent;
use egui_android_runtime::Application;

/// Обработать событие жизненного цикла.
///
/// Единая точка входа для всех lifecycle-событий из главного цикла.
///
/// # Параметры
///
/// * `event` — событие жизненного цикла
/// * `backend` — backend платформы (для InitWindow)
/// * `app_instance` — экземпляр приложения
/// * `egui_ctx` — контекст egui
/// * `egui_painter` — опциональный painter (для InitWindow)
/// * `destroy_requested` — флаг завершения (устанавливается при Destroy)
pub fn handle_lifecycle_event<A: Application>(
    event: LifecycleEvent,
    backend: &mut dyn AndroidBackend,
    app_instance: &mut A,
    egui_ctx: &egui::Context,
    egui_painter: &mut Option<egui_glow::Painter>,
    destroy_requested: &mut bool,
) {
    match event {
        LifecycleEvent::InitWindow => {
            handle_init_window(backend, app_instance, egui_ctx, egui_painter)
        }
        LifecycleEvent::Resume => handle_resume(app_instance),
        LifecycleEvent::Pause => handle_pause(app_instance),
        LifecycleEvent::Stop => handle_stop(app_instance),
        LifecycleEvent::Destroy => handle_destroy(app_instance, destroy_requested),
    }
}

/// Обработать InitWindow — инициализация или пересоздание EGL.
///
/// # Логика
///
/// 1. Проверяет, был ли уже создан EGL (через surface_handle).
/// 2. Если EGL есть — пересоздаёт surface (Pause/Resume cycle).
/// 3. Если EGL нет — инициализирует с нуля + вызывает on_restore_state().
/// 4. В любом случае обновляет insets, DPI и сохраняет PlatformState в egui::Context.
fn handle_init_window<A: Application>(
    backend: &mut dyn AndroidBackend,
    app_instance: &mut A,
    egui_ctx: &egui::Context,
    egui_painter: &mut Option<egui_glow::Painter>,
) {
    log::info!("Lifecycle: InitWindow");

    // Проверяем, был ли уже создан EGL
    let has_egl = !backend.surface_handle().as_egl_surface().is_null();

    if has_egl {
        // InitWindow после Pause/Resume — пересоздаём surface
        if let Err(e) = backend.recreate_surface() {
            log::error!("Ошибка пересоздания EGL surface: {}", e);
            if let Some(ref mut p) = egui_painter {
                p.destroy();
            }
            *egui_painter = None;
            backend.destroy_graphics();
        } else {
            // Повторно инициализируем painter с новым surface
            if let Some(ref mut p) = egui_painter {
                unsafe {
                    use glow::HasContext;
                    let gl = &*p.gl();
                    gl.clear_color(0.0, 0.0, 0.0, 1.0);
                    gl.clear(glow::COLOR_BUFFER_BIT);
                }
            }
        }
    } else {
        // Первый InitWindow — инициализируем EGL
        backend.init().ok();
        if let Err(e) = backend.init_graphics() {
            log::error!("Ошибка инициализации EGL: {}", e);
        }
        // Восстанавливаем состояние навигации после пересоздания
        app_instance.on_restore_state();
    }

    // Обновляем системные отступы через JNI
    backend.update_system_insets();

    // Сохраняем PlatformState в egui::Context для Application
    backend.platform_state().store_in_ctx(egui_ctx);

    // Обновляем DPI
    egui_ctx.set_pixels_per_point(backend.dpi());
}

/// Обработать Resume — приложение возобновило работу.
fn handle_resume<A: Application>(app_instance: &mut A) {
    log::info!("Lifecycle: Resume");
    app_instance.on_resume();
}

/// Обработать Pause — приложение приостановлено.
fn handle_pause<A: Application>(app_instance: &mut A) {
    log::info!("Lifecycle: Pause");
    app_instance.on_pause();
}

/// Обработать Stop — приложение остановлено.
fn handle_stop<A: Application>(app_instance: &mut A) {
    log::info!("Lifecycle: Stop");
    app_instance.on_stop();
}

/// Обработать Destroy — приложение уничтожается.
///
/// Сохраняет состояние навигации и устанавливает флаг завершения.
fn handle_destroy<A: Application>(app_instance: &mut A, destroy_requested: &mut bool) {
    log::info!("Lifecycle: Destroy");
    app_instance.on_save_state();
    *destroy_requested = true;
}
