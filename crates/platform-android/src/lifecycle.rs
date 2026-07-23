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
//! Вызывается из главного цикла в `loop.rs`.
//!
//! # Сохранение состояния (Decompose-style)
//!
//! Platform только передаёт lifecycle-события.
//! Application сам владеет состоянием: `on_save_state()` сохраняет,
//! `on_restore_state(None)` восстанавливает из внутреннего поля.
//!
//! Для kill/restore процесса данные проходят через PlatformState → JNI → Bundle:
//! - Stop/Destroy: `on_save_state()` → `platform_state.set_saved_state()`
//!   → Kotlin `onSaveInstanceState` → JNI `nativeGetSavedState` → Bundle
//! - InitWindow: Bundle → JNI `nativeSetSavedState` → `platform_state.set_saved_state()`
//!   → `on_restore_state(Some(bytes))`

#![cfg(target_os = "android")]

use crate::backend::AndroidBackend;
use crate::event::LifecycleEvent;
use crate::graphics::GraphicsPipeline;
use crate::platform_state::PlatformState;
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
/// * `graphics` — опциональный GraphicsPipeline (для InitWindow: пересоздание surface)
/// * `destroy_requested` — флаг завершения (устанавливается при Destroy)
/// * `platform_state` — состояние платформы (буфер для JNI-моста kill/restore)
pub fn handle_lifecycle_event<A: Application>(
    event: LifecycleEvent,
    backend: &mut dyn AndroidBackend,
    app_instance: &mut A,
    egui_ctx: &egui::Context,
    graphics: &mut Option<GraphicsPipeline>,
    destroy_requested: &mut bool,
    platform_state: &PlatformState,
) {
    match event {
        LifecycleEvent::InitWindow => {
            handle_init_window(backend, app_instance, egui_ctx, graphics, platform_state)
        }
        LifecycleEvent::Resume => handle_resume(app_instance),
        LifecycleEvent::Pause => handle_pause(app_instance),
        LifecycleEvent::Stop => handle_stop(app_instance, platform_state),
        LifecycleEvent::Destroy => handle_destroy(app_instance, destroy_requested, platform_state),
    }
}

/// Обработать InitWindow — инициализация или пересоздание EGL.
///
/// # Логика
///
/// 1. Проверяет, был ли уже создан EGL (через surface_handle).
/// 2. Если EGL есть — пересоздаёт surface (Pause/Resume cycle).
/// 3. Если EGL нет — инициализирует с нуля + вызывает on_restore_state(None).
/// 4. В любом случае обновляет insets, DPI и сохраняет PlatformState в egui::Context.
///
/// # Восстановление состояния
///
/// При kill/restore: Kotlin `onCreate` уже вызвал `nativeSetSavedState`,
/// данные уже в `platform_state.saved_state_buffer`.
/// Забираем их через `take_saved_state()` и передаём в `on_restore_state(Some(bytes))`.
/// Если буфер пуст — первый запуск или config change, вызываем `on_restore_state(None)`
/// (Application восстановит из своего поля).
fn handle_init_window<A: Application>(
    backend: &mut dyn AndroidBackend,
    app_instance: &mut A,
    egui_ctx: &egui::Context,
    graphics: &mut Option<GraphicsPipeline>,
    platform_state: &PlatformState,
) {
    log::info!("Lifecycle: InitWindow");

    // Проверяем, был ли уже создан EGL
    let has_egl = !backend.surface_handle().as_egl_surface().is_null();

    if has_egl {
        // InitWindow после Pause/Resume — пересоздаём surface
        // и восстанавливаем состояние навигации
        if let Err(e) = backend.recreate_surface() {
            log::error!("Ошибка пересоздания EGL surface: {}", e);
            if let Some(ref mut g) = graphics {
                g.destroy();
            }
            *graphics = None;
            backend.destroy_graphics();
        } else if let Some(ref mut g) = graphics {
            unsafe {
                use glow::HasContext;
                let gl = &*g.painter.gl();
                gl.clear_color(0.0, 0.0, 0.0, 1.0);
                gl.clear(glow::COLOR_BUFFER_BIT);
            }
        }
        // Восстанавливаем состояние: сначала пробуем данные из Bundle (kill/restore),
        // затем своё поле (config change)
        let saved = platform_state.take_saved_state();
        log::info!(
            "Lifecycle: InitWindow (has_egl) — saved_state_buffer: {}",
            if saved.is_some() {
                "есть данные"
            } else {
                "пуст"
            }
        );
        app_instance.on_restore_state(saved);
    } else {
        // Первый InitWindow — инициализируем EGL
        backend.init().ok();
        if let Err(e) = backend.init_graphics() {
            log::error!("Ошибка инициализации EGL: {}", e);
        }
        // Восстанавливаем состояние: сначала пробуем данные из Bundle (kill/restore),
        // затем своё поле (config change)
        let saved = platform_state.take_saved_state();
        log::info!(
            "Lifecycle: InitWindow (первый) — saved_state_buffer: {}",
            if saved.is_some() {
                "есть данные"
            } else {
                "пуст"
            }
        );
        app_instance.on_restore_state(saved);
    }

    // Обновляем системные отступы через JNI
    backend.update_system_insets();

    // Настраиваем системные бары: прозрачность + цвет
    let state = backend.platform_state();
    let vm = state.vm_ptr();
    let activity = state.activity_ptr();
    if !vm.is_null() && !activity.is_null() {
        crate::system_bars::set_transparent_system_bars(vm, activity);
    }
    crate::system_bars::apply_system_bars_for_platform_state(state);

    // Применяем clear_color при старте
    let clear = egui::Color32::from_rgb(0x33, 0x33, 0x33);
    backend.platform_state().set_clear_color_from(clear);

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
///
/// Вызывает `on_save_state()` — результат сохраняется в `platform_state`
/// для JNI-моста (kill/restore через Bundle).
/// Application также сохраняет в своё поле для config change.
fn handle_stop<A: Application>(app_instance: &mut A, platform_state: &PlatformState) {
    log::info!("Lifecycle: Stop");
    let saved = app_instance.on_save_state();
    if let Some(bytes) = saved {
        log::info!(
            "Lifecycle: Stop — сохраняем {} байт в platform_state",
            bytes.len()
        );
        platform_state.set_saved_state(bytes);
    }
    app_instance.on_stop();
}

/// Обработать Destroy — приложение уничтожается.
///
/// Вызывает `on_save_state()` (на случай если Stop не был вызван)
/// и устанавливает флаг завершения.
/// Результат сохраняется в `platform_state` для JNI-моста.
fn handle_destroy<A: Application>(
    app_instance: &mut A,
    destroy_requested: &mut bool,
    platform_state: &PlatformState,
) {
    log::info!("Lifecycle: Destroy");
    let saved = app_instance.on_save_state();
    if let Some(bytes) = saved {
        log::info!(
            "Lifecycle: Destroy — сохраняем {} байт в platform_state",
            bytes.len()
        );
        platform_state.set_saved_state(bytes);
    }
    *destroy_requested = true;
}
