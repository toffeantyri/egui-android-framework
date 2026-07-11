//! Главный цикл для Decompose-style архитектуры.
//!
//! Событийный цикл — без polling, без фоновых потоков для UI.
//! `UiNotifier` проверяется синхронно в главном потоке.

#![cfg(target_os = "android")]

use std::sync::Arc;
use std::time::{Duration, Instant};

use android_activity::{AndroidApp, MainEvent, PollEvent};

use crate::egl_backend::EglState;
use crate::input::{self, InputState};

use egui_android_runtime::{
    ui_notifier::{AndroidWakeHandle, UiNotifier},
    Application,
};

use glow::HasContext;

/// Запустить egui-приложение на Android.
pub fn run<A: Application>(app: AndroidApp) {
    let mut app_instance = A::create();

    android_logger::init_once(
        android_logger::Config::default()
            .with_tag(app_instance.config().log_tag.as_str())
            .with_max_level(log::LevelFilter::Debug),
    );
    log::info!("run: запуск egui-android-framework");

    app_instance.on_create();
    app_instance.on_start();
    app_instance.on_resume();

    // Устанавливаем прозрачные системные бары до начала цикла.
    // Используем poll_events с MainEvent::InitWindow, чтобы дождаться окна.
    // Первый InitWindow гарантированно придёт после on_resume.
    let vm_ptr = app.vm_as_ptr() as usize;
    let activity_ptr = app.activity_as_ptr() as usize;
    app.run_on_java_main_thread(Box::new(move || {
        let vm = vm_ptr as *mut std::ffi::c_void;
        let activity = activity_ptr as *mut std::ffi::c_void;
        crate::system_bars::set_transparent_system_bars(vm, activity);
    }));

    let egui_ctx = egui::Context::default();
    let mut egl_state: Option<EglState> = None;
    let mut egui_painter: Option<egui_glow::Painter> = None;

    let mut last_frame = Instant::now();
    let target_dt = Duration::from_secs_f64(1.0 / app_instance.config().target_fps as f64);

    let mut input_state = InputState::new();
    let mut surface_needs_recreation = false;

    // Создаём waker для Android event loop.
    let waker = app.create_waker();
    let wake_handle = AndroidWakeHandle::new(move || {
        waker.wake();
    });
    log::info!("run: Android waker создан");

    // Реактивный уведомитель — создаётся Application после EGL init.
    let mut notifier: Option<UiNotifier> = None;
    let mut notifier_initialized = false;

    let mut destroy_requested = false;
    loop {
        app.poll_events(Some(Duration::from_millis(16)), |event| match event {
            PollEvent::Wake | PollEvent::Timeout => {
                // Wake от data layer или таймаут — идём в рендеринг
            }
            PollEvent::Main(e) => match e {
                MainEvent::InitWindow { .. } => {
                    log::info!("Lifecycle: InitWindow");
                    surface_needs_recreation = true;

                    // После создания окна запрашиваем insets через JNI на главном Java-потоке.
                    // В этот момент DecorView уже существует, getRootWindowInsets() вернёт корректные значения.
                    // Системные бары уже установлены в начале run() — здесь только insets.
                    let vm_ptr = app.vm_as_ptr() as usize;
                    let activity_ptr = app.activity_as_ptr() as usize;
                    app.run_on_java_main_thread(Box::new(move || {
                        let vm = vm_ptr as *mut std::ffi::c_void;
                        let activity = activity_ptr as *mut std::ffi::c_void;
                        if let Some(insets) = crate::insets::get_system_insets_jni(vm, activity) {
                            crate::insets::store_insets(&insets);
                        } else {
                            log::warn!("JNI insets не получены — будет использован fallback");
                        }

                        // Определяем системную тему и сохраняем
                        if let Some(theme) =
                            crate::system_bars::detect_system_theme_jni(vm, activity)
                        {
                            crate::theme::init_system_theme(theme);
                            log::info!("Системная тема: {:?}", theme);
                            // Применяем цвета баров по умолчанию
                            crate::system_bars::apply_system_bars_color_jni(vm, activity, theme);
                        }
                    }));

                    // Флаги set_window_flags не применяем — они ломают рендеринг на MIUI.
                    // ActionBar скрываем через тему @android:style/Theme.Material.Light.NoActionBar.Fullscreen
                    // в AndroidManifest.xml.
                }
                MainEvent::Resume { .. } => {
                    log::info!("Lifecycle: Resume");
                    app_instance.on_resume();
                }
                MainEvent::Pause { .. } => {
                    log::info!("Lifecycle: Pause");
                    app_instance.on_pause();
                }
                MainEvent::Stop { .. } => {
                    log::info!("Lifecycle: Stop");
                    app_instance.on_stop();
                }
                MainEvent::Destroy { .. } => {
                    log::info!("Lifecycle: Destroy");
                    destroy_requested = true;
                }
                _ => {}
            },
            _ => {}
        });

        // --- Ввод: сливаем ВСЕ накопившиеся события ---
        {
            let pp = egui_ctx.pixels_per_point();
            input::process_input_events(&app, pp, &mut input_state);
        }

        if input_state.back_pressed {
            log::info!("Back нажата — отправляем в Application");
            input_state.back_pressed = false;
            app_instance.on_back_pressed();
        }

        if destroy_requested {
            app_instance.on_destroy();
            if let Some(ref mut p) = egui_painter {
                p.destroy();
            }
            egui_painter = None;
            if let Some(ref mut state) = egl_state {
                state.destroy();
            }
            egl_state = None;
            log::info!("Выход из главного цикла (run)");
            break;
        }

        // --- Пересоздание EGL surface ---
        if surface_needs_recreation {
            if let Some(nw) = app.native_window() {
                if let Some(ref mut state) = egl_state {
                    match state.recreate_surface(&nw) {
                        Ok(()) => {
                            if let Some(ref mut painter) = egui_painter {
                                unsafe {
                                    gl_clear(&*painter.gl());
                                }
                            }
                            // При пересоздании surface обновляем pp на случай смены конфигурации
                            let pp = crate::insets::get_pp(&app.config(), nw.width(), nw.height());
                            egui_ctx.set_pixels_per_point(pp);
                        }
                        Err(e) => {
                            log::error!("Не удалось пересоздать EGL surface: {}", e);
                            if let Some(ref mut p) = egui_painter {
                                p.destroy();
                            }
                            egui_painter = None;
                            if let Some(ref mut s) = egl_state {
                                s.destroy();
                            }
                            egl_state = None;
                        }
                    }
                }
            }
            surface_needs_recreation = false;
        }

        // --- Инициализация EGL ---
        if egl_state.is_none() {
            if let Some(nw) = app.native_window() {
                match EglState::create(&nw) {
                    Ok(state) => {
                        let gl = unsafe {
                            egui_glow::painter::Context::from_loader_function(|name| {
                                let cname = std::ffi::CStr::from_ptr(
                                    name.as_ptr() as *const std::os::raw::c_char
                                );
                                get_proc_address(cname)
                            })
                        };

                        match egui_glow::Painter::new(Arc::new(gl), "", None, true) {
                            Ok(painter) => {
                                unsafe {
                                    gl_clear(&*painter.gl());
                                }
                                egui_ctx.set_fonts(egui::FontDefinitions::default());
                                let pp =
                                    crate::insets::get_pp(&app.config(), nw.width(), nw.height());
                                egui_ctx.set_pixels_per_point(pp);
                                egl_state = Some(state);
                                egui_painter = Some(painter);

                                if !notifier_initialized {
                                    notifier_initialized = true;
                                    log::info!("run: инициализируем UiNotifier");
                                    notifier = app_instance
                                        .create_notifier(&egui_ctx, wake_handle.clone());
                                }
                            }
                            Err(e) => {
                                log::error!("Painter::new не удался: {:?}", e);
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("EglState::create не удался: {}", e);
                    }
                }
            }
        }

        // --- Проверка уведомлений от data layer ---
        if let Some(ref mut n) = notifier {
            n.check();
        }

        // --- Рендеринг ---
        let now = Instant::now();
        if now.duration_since(last_frame) >= target_dt {
            last_frame = now;

            if let (Some(ref mut painter), Some(ref state)) = (&mut egui_painter, &egl_state) {
                let (w, h) = app
                    .native_window()
                    .map(|nw| (nw.width() as u32, nw.height() as u32))
                    .unwrap_or((0, 0));
                if w == 0 || h == 0 {
                    continue;
                }

                let content_rect = app.content_rect();
                let insets =
                    crate::insets::get_insets_pt(&app.config(), w as i32, h as i32, &content_rect);

                let pp = egui_ctx.pixels_per_point();

                #[cfg(debug_assertions)]
                {
                    use std::time::{Duration, Instant};
                    static LAST_BATCH_LOG: std::sync::OnceLock<Instant> =
                        std::sync::OnceLock::new();
                    let now = Instant::now();
                    let should_log = LAST_BATCH_LOG.get().map_or(true, |last| {
                        now.duration_since(*last) >= Duration::from_secs(5)
                    });
                    if should_log {
                        let _ = LAST_BATCH_LOG.set(now);
                        if LAST_BATCH_LOG.get().map_or(false, |last| {
                            now.duration_since(*last) < Duration::from_secs(5)
                        }) {
                            // set успешен — первый раз
                        } else if let Some(last) = LAST_BATCH_LOG.get() {
                            if now.duration_since(*last) >= Duration::from_secs(5) {
                                // Обновляем через OnceLock нельзя, используем атомик
                            }
                        }
                    }
                }
                #[cfg(debug_assertions)]
                {
                    // Логируем раз в 5 секунд
                    use std::sync::atomic::{AtomicU64, Ordering};
                    use std::time::{SystemTime, UNIX_EPOCH};
                    static LAST_LOG_MS: AtomicU64 = AtomicU64::new(0);
                    let now_ms = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;
                    let last = LAST_LOG_MS.load(Ordering::Relaxed);
                    if now_ms.saturating_sub(last) > 5000 || last == 0 {
                        LAST_LOG_MS.store(now_ms, Ordering::Relaxed);
                        log::warn!(
                            "[BATCH] native_size=({},{}) pp={} | insets=left:{:.1} top:{:.1} right:{:.1} bottom:{:.1}",
                            w, h, pp,
                            insets.left, insets.top, insets.right, insets.bottom
                        );
                    }
                }

                // Все события в кадре — батчинг не нужен, патч egui сам обнуляет
                // pointer.delta() на первом кадре нового drag.
                let events_for_frame = std::mem::take(&mut input_state.events);

                let screen_rect = egui::Rect::from_min_size(
                    egui::Pos2::new(insets.left, insets.top),
                    egui::vec2(
                        (w as f32 / pp) - insets.left - insets.right,
                        (h as f32 / pp) - insets.top - insets.bottom,
                    ),
                );
                // Время с предыдущего кадра — для стабильной децелерации fling.
                // Используем elapsed, а не Instant::now() — egui внутри пересчитывает dt.
                let predicted_dt = target_dt.as_secs_f64() as f32;
                // Устанавливаем time = Instant::now() как f64.
                // Без этого egui использует self.time + predicted_dt и время между кадрами
                // может быть неправильным, что вызывает дёрганье скролла при повторном касании.
                let time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs_f64();
                let raw_input = egui::RawInput {
                    screen_rect: Some(screen_rect),
                    events: events_for_frame,
                    predicted_dt,
                    time: Some(time),
                    ..egui::RawInput::default()
                };

                let full_output = app_instance.frame(&egui_ctx, raw_input);

                // Проверяем, не запросил ли Application завершение
                if app_instance.request_destroy() {
                    destroy_requested = true;
                }

                unsafe {
                    let gl = &*painter.gl();

                    // 1. Очищаем ВЕСЬ framebuffer цветом фона (под системными барами).
                    // Прозрачность (alpha=0) не работает на MIUI/NativeActivity —
                    // SurfaceFlinger рисует чёрный. Поэтому задаём явный цвет.
                    gl.disable(glow::SCISSOR_TEST);
                    let (cr, cg, cb) = crate::theme::current_clear_color();
                    gl.clear_color(cr, cg, cb, 1.0);
                    gl.clear(glow::COLOR_BUFFER_BIT);

                    // 2. Включаем scissor — egui_glow будет рисовать только внутри content_rect.
                    // Области под барами остаются с цветом из clear_color выше.
                    let scissor_x = (insets.left * pp) as i32;
                    let scissor_y = (insets.bottom * pp) as i32;
                    let scissor_w = (w as f32 - insets.left * pp - insets.right * pp) as i32;
                    let scissor_h = (h as f32 - insets.top * pp - insets.bottom * pp) as i32;
                    gl.enable(glow::SCISSOR_TEST);
                    gl.scissor(scissor_x, scissor_y, scissor_w.max(1), scissor_h.max(1));
                }

                let primitives =
                    egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);

                painter.paint_and_update_textures(
                    [w, h],
                    full_output.pixels_per_point,
                    &primitives,
                    &full_output.textures_delta,
                );

                if let Err(e) = state.swap_buffers() {
                    log::warn!("swap_buffers: {}", e);
                    if let Some(ref mut p) = egui_painter {
                        p.destroy();
                    }
                    egui_painter = None;
                    egl_state = None;
                }
            }
        }
    }
}

unsafe fn gl_clear(gl: &glow::Context) {
    gl.clear_color(0.0, 0.0, 0.0, 0.0);
    gl.clear(glow::COLOR_BUFFER_BIT);
}

fn get_proc_address(name: &std::ffi::CStr) -> *const std::os::raw::c_void {
    type EglGetProcAddress =
        unsafe extern "system" fn(*const std::os::raw::c_char) -> *const std::os::raw::c_void;

    let handle = unsafe {
        libc::dlopen(
            "libEGL.so\0".as_ptr() as *const std::os::raw::c_char,
            libc::RTLD_LAZY | libc::RTLD_NOLOAD,
        )
    };
    if handle.is_null() {
        log::error!("get_proc_address: libEGL.so не загружен");
        return std::ptr::null();
    }

    let sym = unsafe {
        libc::dlsym(
            handle,
            "eglGetProcAddress\0".as_ptr() as *const std::os::raw::c_char,
        )
    };
    if sym.is_null() {
        log::error!("get_proc_address: eglGetProcAddress не найден");
        return std::ptr::null();
    }

    let func: EglGetProcAddress = unsafe { std::mem::transmute(sym) };
    let addr = unsafe { func(name.as_ptr()) };

    if addr.is_null() {
        let handle2 = unsafe {
            libc::dlopen(
                "libGLESv2.so\0".as_ptr() as *const std::os::raw::c_char,
                libc::RTLD_LAZY | libc::RTLD_NOLOAD,
            )
        };
        if let Ok(s) = name.to_str() {
            log::warn!("eglGetProcAddress вернул null для {}", s);
        }
        if !handle2.is_null() {
            let fallback = unsafe { libc::dlsym(handle2, name.as_ptr()) };
            if !fallback.is_null() {
                return fallback;
            }
        }
    }
    addr
}
