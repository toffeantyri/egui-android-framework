//! Главный цикл приложения для Android: обработка жизненного цикла,
//! touch-ввод, инициализация EGL/egui и рендеринг.
//!
//! Точка входа — [`run()`], которая принимает конкретный тип,
//! реализующий [`Application`].

#![cfg(target_os = "android")]

use std::sync::Arc;
use std::time::{Duration, Instant};

use android_activity::{AndroidApp, MainEvent, PollEvent};

use crate::activity::Activity;
use crate::android::egl_backend::EglState;
use crate::android::input::{self, InputState};
use crate::{AppContext, Application, LifecycleObserver, ViewModel};

use glow::HasContext;

/// Запустить egui-приложение на Android.
///
/// Вызывается из `#[no_mangle] pub fn android_main(app: AndroidApp)` в крейте пользователя,
/// которая настраивает конкретный тип приложения.
pub fn run<A: Application>(app: AndroidApp) {
    let mut app_ctx = AppContext::<A>::new();
    A::on_create(&mut app_ctx);

    // Инициализируем логгер с тегом из конфига приложения
    android_logger::init_once(
        android_logger::Config::default()
            .with_tag(app_ctx.config().log_tag.as_str())
            .with_max_level(log::LevelFilter::Info),
    );
    log::info!("run: запуск egui-android-framework приложения");

    // create_view_model вызывается первой — она сама создаёт каналы
    // через ctx.view_model_context(). Затем run() получает свою копию vm_ctx
    // с тем же Receiver (через Arc), чтобы опрашивать события из data layer.
    let mut view_model = A::create_view_model(&mut app_ctx);
    let vm_ctx = app_ctx.view_model_context();
    let mut activity = A::create_activity(&mut app_ctx);

    activity.on_create();
    activity.on_start();

    let egui_ctx = egui::Context::default();
    let mut egl_state: Option<EglState> = None;
    let mut egui_painter: Option<egui_glow::Painter> = None;

    let mut needs_redraw = true;
    let mut last_frame = Instant::now();
    let target_dt = Duration::from_secs_f64(1.0 / app_ctx.config().target_fps as f64);

    // Состояние ввода (touch, клавиатура)
    let mut input_state = InputState::new();

    // Флаг: EGL display существует, но surface устарел из-за InitWindow.
    // Устанавливается в колбэке InitWindow, сбрасывается после пересоздания surface.
    let mut surface_needs_recreation = false;

    loop {
        input_state.clear();

        // --- Обработка событий жизненного цикла ---
        let mut destroy_requested = false;
        app.poll_events(None, |event| match event {
            PollEvent::Wake | PollEvent::Timeout => {}
            PollEvent::Main(e) => match e {
                MainEvent::InitWindow { .. } => {
                    log::info!("Lifecycle: InitWindow — планирование пересоздания surface");
                    surface_needs_recreation = true;
                    needs_redraw = true;
                }
                MainEvent::Resume { .. } => {
                    log::info!("Lifecycle: Resume");
                    activity.on_resume();
                    needs_redraw = true;
                }
                MainEvent::Pause { .. } => {
                    log::info!("Lifecycle: Pause");
                    activity.on_pause();
                }
                MainEvent::Stop { .. } => {
                    log::info!("Lifecycle: Stop — сохранение состояния EGL");
                    activity.on_stop();
                }
                MainEvent::Destroy { .. } => {
                    log::info!("Lifecycle: Destroy — выход");
                    destroy_requested = true;
                }
                MainEvent::RedrawNeeded { .. } => {
                    needs_redraw = true;
                }
                _ => {}
            },
            _ => {}
        });

        // --- Ввод (touch, клавиатура, back) ---
        {
            let pp = egui_ctx.pixels_per_point();
            input::process_input_events(&app, pp, &mut input_state);
        }

        // --- Обработка кнопки Back (должна быть ДО проверки destroy_requested) ---
        if input_state.back_pressed {
            if activity.on_back_pressed(&mut view_model) {
                log::info!("Back нажата — выход");
                destroy_requested = true;
            } else {
                log::info!("Back нажата — не обработано, игнорируем");
            }
        }

        if destroy_requested {
            activity.on_destroy();
            if let Some(ref mut p) = egui_painter {
                p.destroy();
            }
            egui_painter = None;
            if let Some(ref mut state) = egl_state {
                state.destroy();
            }
            egl_state = None;
            log::info!("Выход из главного цикла — android_main вернёт управление");
            break;
        }

        // --- Пересоздание EGL surface при смене окна (InitWindow) ---
        if surface_needs_recreation {
            if let Some(nw) = app.native_window() {
                if let Some(ref mut state) = egl_state {
                    log::info!(
                        "Пересоздание EGL surface для нового окна {}x{}",
                        nw.width(),
                        nw.height()
                    );
                    match state.recreate_surface(&nw) {
                        Ok(()) => {
                            if let Some(ref mut painter) = egui_painter {
                                unsafe {
                                    gl_clear(painter.gl());
                                }
                            }
                            let pp = compute_pixels_per_point(nw.width(), nw.height());
                            egui_ctx.set_pixels_per_point(pp);
                            log::info!("EGL surface успешно пересоздан, pp={}", pp);
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

        // --- Инициализация EGL с нуля (первый запуск или после полной очистки) ---
        if egl_state.is_none() {
            if let Some(nw) = app.native_window() {
                log::info!(
                    "Инициализация EGL + egui... размер={}x{}",
                    nw.width(),
                    nw.height()
                );
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
                                log::info!("EGL + egui painter инициализирован! pp={}", pp);

                                egl_state = Some(state);
                                egui_painter = Some(painter);
                                needs_redraw = true;
                            }
                            Err(e) => {
                                log::error!("egui_glow::Painter::new не удался: {:?}", e);
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("EglState::create не удался: {}", e);
                    }
                }
            }
        }

        // --- События из data layer (ViewModelContext::poll_events) ---
        {
            // Отладка: проверка валидности vm_ctx
            let events = vm_ctx.poll_events();
            if !events.is_empty() {
                log::info!("run: получено {} событий из data layer", events.len());
                for evt in events {
                    view_model.on_event(evt);
                }
                needs_redraw = true;
            } else {
                // Молча: в этом кадре нет событий — нормально
            }
        }

        // --- Рендеринг ---
        // Всегда запрашиваем repaint — иначе UI не обновляется при событиях
        // от data layer, и анимации не работают.
        needs_redraw = true;
        let now = Instant::now();
        if needs_redraw && now.duration_since(last_frame) >= target_dt {
            needs_redraw = false;
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
                let top_inset = 45.0_f32.min(h as f32 / pp * 0.06);
                let bottom_inset = 48.0_f32.min(h as f32 / pp * 0.06);

                let raw_input = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::new(0.0, top_inset),
                        egui::vec2(w as f32 / pp, (h as f32 / pp) - top_inset - bottom_inset),
                    )),
                    events: input_state.events.clone(),
                    ..egui::RawInput::default()
                };

                let mut commands: Vec<<A::ViewModel as ViewModel>::DataCommand> = Vec::new();
                let full_output = egui_ctx.run(raw_input, |ctx| {
                    commands = activity.render(ctx, &view_model);
                });

                for cmd in commands {
                    view_model.dispatch(cmd);
                }

                unsafe {
                    let gl = painter.gl();
                    gl.clear_color(0.2, 0.2, 0.2, 1.0);
                    gl.clear(glow::COLOR_BUFFER_BIT);
                }

                let primitives: Vec<egui::epaint::ClippedPrimitive> =
                    egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
                painter.paint_and_update_textures(
                    [w, h],
                    full_output.pixels_per_point,
                    &primitives,
                    &full_output.textures_delta,
                );

                if let Err(e) = state.swap_buffers() {
                    log::warn!("ошибка swap_buffers: {}", e);
                    if let Some(ref mut p) = egui_painter {
                        p.destroy();
                    }
                    egui_painter = None;
                    egl_state = None;
                }
            }
        }

        std::thread::sleep(Duration::from_millis(1));
    }
}

/// Вычислить разумный pixels_per_point из размеров native window в пикселях.
fn compute_pixels_per_point(width: i32, height: i32) -> f32 {
    let w = width.max(height) as f32;
    (w / 400.0).max(1.0).min(5.0)
}

/// Очистить GL буфер (чёрный прозрачный).
unsafe fn gl_clear(gl: &glow::Context) {
    gl.clear_color(0.0, 0.0, 0.0, 0.0);
    gl.clear(glow::COLOR_BUFFER_BIT);
}

/// Получить адрес OpenGL функции через eglGetProcAddress.
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
