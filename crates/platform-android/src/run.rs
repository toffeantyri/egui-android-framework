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

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use android_activity::AndroidApp;

use crate::backend::{AndroidBackend, AndroidBackendKind};
use crate::r#loop::RunState;

use egui_android_runtime::{keyboard_controller_id, Application, KeyboardController};

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
    // Клон AndroidApp для KeyboardController: компилятор не позволяет заимствовать
    // его из backend после регистрации в egui Context (нужно 'static для closures).
    let app_for_keyboard = app.clone();
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

    // Системные бары настраиваются в handle_init_window()
    // после инициализации backend'а, когда PlatformState уже
    // содержит валидные JNI GlobalRef.

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

    // Флаг ime_visible — разделяется с KeyboardController (Arc<AtomicBool>).
    // GlBackend хранит его, KeyboardController обновляет при show/hide.
    let ime_flag = if backend.supports_ime() {
        // Безопасно: supports_ime() → это GlBackend, который хранит Arc<AtomicBool>.
        // Через unsafe downcast до конкретного типа (единственный вариант без трейт-метода).
        let gl = unsafe {
            &*(&*backend as *const dyn AndroidBackend as *const crate::backend::GlBackend)
        };
        gl.ime_visible_flag()
    } else {
        Arc::new(AtomicBool::new(false))
    };

    let egui_ctx = egui::Context::default();
    egui_ctx.set_pixels_per_point(backend.dpi());
    egui_ctx.set_fonts(egui::FontDefinitions::default());

    // Регистрируем контроллер клавиатуры (IME) в egui Context data.
    // Виджет TextEdit читает его по событию фокуса и вызывает show()/hide().
    // Если backend не поддерживает IME (NativeBackend) — пропускаем.
    if backend.supports_ime() {
        let app_show = app_for_keyboard.clone();
        let app_hide = app_for_keyboard.clone();
        let show_flag = Arc::clone(&ime_flag);
        let hide_flag = Arc::clone(&ime_flag);
        let kb = KeyboardController::new(
            Arc::new(move || {
                log::info!("KeyboardController: показать клавиатуру");
                show_flag.store(true, Ordering::Relaxed);
                app_show.show_soft_input(false);
            }),
            Arc::new(move || {
                log::info!("KeyboardController: скрыть клавиатуру");
                hide_flag.store(false, Ordering::Relaxed);
                app_hide.hide_soft_input(false);
            }),
        );
        egui_ctx.data_mut(|d| {
            d.insert_temp(keyboard_controller_id(), kb);
        });
        log::info!("KeyboardController: зарегистрирован в egui Context");
    }

    let waker = backend.create_waker();

    let mut state = RunState::new();
    let target_dt = Duration::from_secs_f64(1.0 / app_instance.config().target_fps as f64);

    // Инициализируем JNI-мост для kill/restore — регистрируем PlatformState
    // в глобальном OnceLock для доступа из nativeGetSavedState/nativeSetSavedState.
    let platform_state = backend.platform_state().clone();
    crate::saved_state_jni::init_jni_platform_state(platform_state.clone());

    // --- Главный цикл ---
    while state.tick(
        &mut *backend,
        &mut app_instance,
        &egui_ctx,
        &waker,
        target_dt,
        &platform_state,
    ) {}

    // --- Очистка ---
    if let Some(ref mut g) = state.graphics {
        g.destroy();
    }
    backend.destroy_graphics();
    log::info!("Выход из главного цикла");
}
