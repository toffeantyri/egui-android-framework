//! Главный цикл — оркестратор platform-android.
//!
//! Единственная точка входа для запуска egui-приложения на Android.
//! Вызывается из `android_main()` в каждом приложении.
//!
//! # Архитектура
//!
//! `run_with_backend()` выполняет три фазы:
//!
//! 1. **Инициализация**
//!    - Создание `Application` (DI-корень)
//!    - Создание backend (`GlBackend` / `NativeBackend`)
//!    - Настройка системных баров (прозрачные, JNI)
//!    - Создание `egui::Context`, Waker
//!    - Создание `RunState`
//!
//! 2. **Главный цикл** — `RunState::tick()` (см. [`loop::RunState`])
//!    - `poll_events` → lifecycle / input_processing
//!    - `destroy_requested` → on_destroy + break
//!    - `GraphicsPipeline::try_new()`
//!    - `back_pressed` → process_back_pressed
//!    - `rt_ctx.check()`
//!    - render: insets → raw_input → frame → render_frame
//!
//! 3. **Очистка**
//!    - `GraphicsPipeline::destroy()`
//!    - `backend.destroy_graphics()`
//!
//! # Событийный цикл
//!
//! `RunState::tick()` — одна итерация. Всегда без блокировки:
//! - `poll_events(0ms)` — неблокирующий опрос
//! - `UiNotifier` — сигнал от data layer, проверяется каждый кадр
//! - FPS-ограничение через `target_dt` (60 FPS по умолчанию)

#![cfg(target_os = "android")]

use std::time::Duration;

use android_activity::AndroidApp;

use crate::backend::{AndroidBackend, AndroidBackendKind};
use crate::r#loop::RunState;

use egui_android_runtime::Application;

/// Запустить egui-приложение на Android.
///
/// Использует `GlBackend` (основной).
pub fn run<A: Application>(app: AndroidApp) {
    run_with_backend::<A>(app, AndroidBackendKind::Gl);
}

/// Запустить egui-приложение с указанным backend'ом.
///
/// `kind` — тип backend'а:
/// - `Gl` — GameActivity + EGL (основной, с IME)
/// - `Native` — NativeActivity (fallback, без IME)
/// - `Game` — зарезервировано (пока использует Gl)
pub fn run_with_backend<A: Application>(app: AndroidApp, kind: AndroidBackendKind) {
    let mut app_instance = A::create();

    android_logger::init_once(
        android_logger::Config::default()
            .with_tag(app_instance.config().log_tag.as_str())
            .with_max_level(log::LevelFilter::Debug),
    );
    log::info!("run: запуск egui-android-framework, backend={:?}", kind);

    app_instance.on_create();
    app_instance.on_start();
    app_instance.on_resume();

    // --- Создаём backend и настраиваем системные бары ---
    // System bars (setStatusBarColor/setNavigationBarColor) — только на UI thread.
    // Захватываем указатели до перемещения app в backend.
    let vm_ptr = app.vm_as_ptr() as usize;
    let activity_ptr = app.activity_as_ptr() as usize;
    app.run_on_java_main_thread(Box::new(move || {
        let vm = vm_ptr as *mut std::ffi::c_void;
        let activity = activity_ptr as *mut std::ffi::c_void;
        if !vm.is_null() && !activity.is_null() {
            crate::system_bars::set_transparent_system_bars(vm, activity);
        }
    }));

    let mut backend: Box<dyn AndroidBackend> = match kind {
        AndroidBackendKind::Gl => {
            log::info!("run: создаём GlBackend");
            Box::new(crate::backend::GlBackend::new(app))
        }
        AndroidBackendKind::Native => {
            log::info!("run: создаём NativeBackend (fallback)");
            Box::new(crate::backend::NativeBackend::new(app))
        }
        AndroidBackendKind::Game => {
            log::warn!("GameBackend ещё не реализован, используем GlBackend");
            Box::new(crate::backend::GlBackend::new(app))
        }
    };

    let egui_ctx = egui::Context::default();
    egui_ctx.set_pixels_per_point(backend.dpi());
    egui_ctx.set_fonts(egui::FontDefinitions::default());

    let waker = backend.create_waker();

    let mut state = RunState::new();
    let target_dt = Duration::from_secs_f64(1.0 / app_instance.config().target_fps as f64);

    // --- Главный цикл ---
    while state.tick(
        &mut *backend,
        &mut app_instance,
        &egui_ctx,
        &waker,
        target_dt,
    ) {}

    // --- Очистка ---
    if let Some(ref mut g) = state.graphics {
        g.destroy();
    }
    backend.destroy_graphics();
    log::info!("Выход из главного цикла");
}
