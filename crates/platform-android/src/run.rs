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

    let egui_ctx = egui::Context::default();
    let mut egl_state: Option<EglState> = None;
    let mut egui_painter: Option<egui_glow::Painter> = None;

    let mut last_frame = Instant::now();
    let target_dt = Duration::from_secs_f64(1.0 / app_instance.config().target_fps as f64);

    let mut input_state = InputState::new();
    let mut surface_needs_recreation = false;
    let mut deferred_events: Vec<egui::Event> = Vec::new();

    // Создаём waker для Android event loop.
    let waker = app.create_waker();
    let wake_handle = AndroidWakeHandle::new(move || {
        waker.wake();
    });
    log::info!("run: Android waker создан");

    // Реактивный уведомитель — создаётся Application после EGL init.
    let mut notifier: Option<UiNotifier> = None;
    let mut notifier_initialized = false;

    loop {
        // --- Слив системных событий ---
        let mut destroy_requested = false;
        app.poll_events(Some(Duration::from_millis(16)), |event| match event {
            PollEvent::Wake | PollEvent::Timeout => {
                // Wake от data layer или таймаут — идём в рендеринг
            }
            PollEvent::Main(e) => match e {
                MainEvent::InitWindow { .. } => {
                    log::info!("Lifecycle: InitWindow");
                    surface_needs_recreation = true;
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
            log::info!("Back нажата — выход");
            destroy_requested = true;
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
                                    gl_clear(painter.gl());
                                }
                            }
                            let pp = compute_pixels_per_point(nw.width(), nw.height());
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
                            glow::Context::from_loader_function(|name| {
                                let cname = std::ffi::CStr::from_ptr(
                                    name.as_ptr() as *const std::os::raw::c_char
                                );
                                get_proc_address(cname)
                            })
                        };

                        match egui_glow::Painter::new(Arc::new(gl), "", None, true) {
                            Ok(painter) => {
                                unsafe {
                                    gl_clear(painter.gl());
                                }
                                egui_ctx.set_fonts(egui::FontDefinitions::default());
                                let pp = compute_pixels_per_point(nw.width(), nw.height());
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

                let pp = egui_ctx.pixels_per_point();
                let content_rect = app.content_rect();
                // content_rect.top на MIUI включает DisplayCutout (334px вместо 152px).
                // content_rect.bottom работает корректно.
                let top_inset = crate::insets::get_top_inset_pt(h as i32, content_rect.top, pp);
                let bottom_inset = (h as i32 - content_rect.bottom) as f32 / pp;

                #[cfg(debug_assertions)]
                log::warn!(
                    "[BATCH] content_rect raw: left={} top={} right={} bottom={} | native_size=({},{}) pp={} | insets=top:{:.0}(fixed) bot:{:.0}(from-content)",
                    content_rect.left, content_rect.top, content_rect.right, content_rect.bottom,
                    w, h, pp,
                    top_inset, bottom_inset
                );

                // ─── Умная упаковка событий в кадр ─────────────────────────
                let mut new_events = std::mem::take(&mut input_state.events);

                if !deferred_events.is_empty() {
                    let mut prev = std::mem::take(&mut deferred_events);
                    prev.append(&mut new_events);
                    new_events = prev;
                }

                let has_down = new_events
                    .iter()
                    .any(|e| matches!(e, egui::Event::PointerButton { pressed: true, .. }));

                let (events_for_frame, split_info) = if has_down {
                    // Если в кадре есть Down — откладываем ВСЕ события, оставляя
                    // только PointerButton { pressed: true } в текущем кадре.
                    // Иначе pointer.delta() на первом кадре drag будет содержать
                    // дельту между latest_pos (от UP) и позицией Move, пришедшего
                    // в том же кадре что и Down, что вызывает мгновенный скачок.
                    let mut events_for_frame: Vec<egui::Event> = Vec::new();
                    let mut deferred_this_frame: Vec<egui::Event> = Vec::new();
                    for e in new_events.drain(..) {
                        if matches!(&e, egui::Event::PointerButton { pressed: true, .. }) {
                            // Down идёт в кадр, но все события ДО него уже в deferred_events
                            events_for_frame.push(e);
                        } else {
                            deferred_this_frame.push(e);
                        }
                    }
                    // Добавляем отложенные этого кадра к глобальным deferred
                    deferred_events.extend(deferred_this_frame);
                    let deferred_count = deferred_events.len();
                    (events_for_frame, Some(deferred_count))
                } else {
                    (new_events, None)
                };

                #[cfg(debug_assertions)]
                {
                    let down_c = events_for_frame
                        .iter()
                        .filter(|e| matches!(e, egui::Event::PointerButton { pressed: true, .. }))
                        .count();
                    let up_c = events_for_frame
                        .iter()
                        .filter(|e| matches!(e, egui::Event::PointerButton { pressed: false, .. }))
                        .count();
                    let move_c = events_for_frame
                        .iter()
                        .filter(|e| matches!(e, egui::Event::PointerMoved(_)))
                        .count();
                    if down_c > 0 || up_c > 0 || move_c > 0 {
                        log::warn!(
                            "[BATCH] Кадр: Down={} Move={} Up={} | отложено={} | всего_в_кадре={} | deferred_events={}",
                            down_c,
                            move_c,
                            up_c,
                            split_info.unwrap_or(0),
                            events_for_frame.len(),
                            deferred_events.len(),
                        );
                        log::warn!(
                            "[BATCH] Viewport: rect=(0,{:.0}),({:.0},{:.0}) pp={} | native=({},{}) | insets=top:{:.0}(px:{}) bot:{:.0}(px:{})",
                            top_inset,
                            w as f32 / pp,
                            (h as f32 / pp) - top_inset - bottom_inset,
                            pp,
                            w,
                            h,
                            top_inset,
                            (top_inset * pp) as i32,
                            bottom_inset,
                            (bottom_inset * pp) as i32
                        );
                    }
                }

                let screen_rect = egui::Rect::from_min_size(
                    egui::Pos2::new(0.0, top_inset),
                    egui::vec2(w as f32 / pp, (h as f32 / pp) - top_inset - bottom_inset),
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

                unsafe {
                    let gl = painter.gl();
                    gl.clear_color(0.2, 0.2, 0.2, 1.0);
                    gl.clear(glow::COLOR_BUFFER_BIT);
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

fn compute_pixels_per_point(width: i32, height: i32) -> f32 {
    // Диагональ экрана в пикселях, нормализация к 400pt базе
    // На современных телефонах (1080p-1440p) даёт pp ≈ 2.5..3.5
    let w = width.max(height) as f32;
    let pp = (w / 450.0).max(1.5).min(4.0);
    log::info!("compute_pixels_per_point: {}x{} → pp={}", width, height, pp);
    pp
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
