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

use std::sync::Arc;
use std::time::Duration;

use android_activity::AndroidApp;

use crate::backend::{AndroidBackend, AndroidBackendKind};
use crate::r#loop::RunState;

use egui_android_runtime::{keyboard_controller_id, Application, KeyboardController};

/// Запустить egui-приложение на Android.
///
/// Использует `GlBackend` (GameActivity). IME работает через собственный
/// невидимый `EguiImeView` (InputConnection → JNI → Rust), а не через
/// штатный IME-протокол GameActivity (TextEvent/setTextInputState).
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

    let egui_ctx = egui::Context::default();
    egui_ctx.set_pixels_per_point(backend.dpi());
    egui_ctx.set_fonts(egui::FontDefinitions::default());

    let waker = backend.create_waker();

    let mut state = RunState::new();
    let target_dt = Duration::from_secs_f64(1.0 / app_instance.config().target_fps as f64);

    // Инициализируем JNI-мост для kill/restore — регистрируем PlatformState
    // в глобальном OnceLock для доступа из nativeGetSavedState/nativeSetSavedState.
    let platform_state = backend.platform_state().clone();
    crate::saved_state_jni::init_jni_platform_state(platform_state.clone());

    // Регистрируем контроллер клавиатуры (IME) в egui Context data.
    //
    // Виджет TextEdit вызывает `KeyboardController.show()/hide()` по фокусу.
    // Фактическое показ/скрытие клавиатуры НЕ происходит здесь — оно
    // централизовано в `loop.rs` по `platform_output.ime` (как в референсе).
    // Замыкания — no-op с логом, чтобы не конфликтовать с основным механизмом.
    //
    // Показ через невидимый EguiImeView (JNI), НЕ через штатный IME GameActivity
    // (InputEvent::TextEvent / setTextInputState).
    if backend.supports_ime() {
        let kb = KeyboardController::new(
            Arc::new(move || {
                // Управление клавиатурой делает loop.rs по `platform_output.ime`.
                log::info!(
                    "KeyboardController.show(): запрошено (управляет loop по platform_output.ime)"
                );
            }),
            Arc::new(move || {
                log::info!(
                    "KeyboardController.hide(): запрошено (управляет loop по platform_output.ime)"
                );
            }),
        );
        egui_ctx.data_mut(|d| {
            d.insert_temp(keyboard_controller_id(), kb);
        });
        log::info!("KeyboardController: зарегистрирован в egui Context");
    }

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
