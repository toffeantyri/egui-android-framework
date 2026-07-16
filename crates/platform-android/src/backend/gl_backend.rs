//! GL-режим backend.
//!
//! Использует `android_activity::AndroidApp` с NativeActivity.
//! IME (клавиатурный ввод) через `run_on_java_main_thread`.
//! EGL + EglState для рендеринга (без изменений рендерера).
//!
//! # Архитектура
//!
//! `GlBackend` хранит `AndroidApp` и EGL-состояние отдельно от
//! событийного буфера, чтобы избежать конфликтов borrow checker
//! при работе с `poll_events()` и `input_events_iter()`.

#![cfg(target_os = "android")]

use std::sync::atomic::{AtomicBool, Ordering};

use android_activity::{
    input::{InputEvent, KeyAction, MotionAction},
    AndroidApp, InputStatus, MainEvent,
};

use crate::backend::{
    AndroidBackend, BackendEvent, InputEvent as BackendInputEvent, Insets,
    KeyAction as BackendKeyAction, LifecycleEvent, SurfaceHandle, TouchPhase,
};
use crate::egl_backend::EglState;

/// GL-режим backend.
pub struct GlBackend {
    app: AndroidApp,
    egl: Option<EglState>,
    events: Vec<BackendEvent>,
    insets: Insets,
    dpi: f32,
    should_close: AtomicBool,
}

impl GlBackend {
    /// Создать новый GL-backend.
    pub fn new(app: AndroidApp) -> Self {
        Self {
            app,
            egl: None,
            events: Vec::new(),
            insets: Insets::default(),
            dpi: 1.0,
            should_close: AtomicBool::new(false),
        }
    }

    /// Получить ссылку на AndroidApp.
    pub fn app(&self) -> &AndroidApp {
        &self.app
    }

    /// Слить lifecycle события.
    fn drain_lifecycle_events(&mut self) {
        let events = &mut self.events;
        let app = &self.app;

        app.poll_events(Some(std::time::Duration::from_millis(0)), |event| {
            match event {
                android_activity::PollEvent::Wake | android_activity::PollEvent::Timeout => {}
                android_activity::PollEvent::Main(e) => match e {
                    MainEvent::InitWindow { .. } => {
                        log::info!("GlBackend: InitWindow");
                        events.push(BackendEvent::Lifecycle(LifecycleEvent::InitWindow));
                        if let Some(nw) = app.native_window() {
                            let pp = crate::insets::get_pp(&app.config(), nw.width(), nw.height());
                            events.push(BackendEvent::DpiChanged(pp));
                        }
                    }
                    MainEvent::Resume { .. } => {
                        log::info!("GlBackend: Resume");
                        events.push(BackendEvent::Lifecycle(LifecycleEvent::Resume));
                    }
                    MainEvent::Pause { .. } => {
                        log::info!("GlBackend: Pause");
                        events.push(BackendEvent::Lifecycle(LifecycleEvent::Pause));
                    }
                    MainEvent::Stop { .. } => {
                        log::info!("GlBackend: Stop");
                        events.push(BackendEvent::Lifecycle(LifecycleEvent::Stop));
                    }
                    MainEvent::Destroy { .. } => {
                        log::info!("GlBackend: Destroy");
                        events.push(BackendEvent::Lifecycle(LifecycleEvent::Destroy));
                    }
                    _ => {}
                },
                _ => {}
            }
        });
    }

    /// Слить input события.
    fn drain_input_events(&mut self) {
        let events = &mut self.events;
        let pp = self.dpi;

        if let Ok(mut iter) = self.app.input_events_iter() {
            loop {
                let has = iter.next(|event| {
                    match event {
                        InputEvent::MotionEvent(motion) => {
                            let action = motion.action();
                            let pointer = motion.pointers().next();
                            match (action, pointer) {
                                (MotionAction::Down, Some(p))
                                | (MotionAction::PointerDown, Some(p)) => {
                                    let pos = egui::pos2(p.x() / pp, p.y() / pp);
                                    events.push(BackendEvent::Input(BackendInputEvent::Touch {
                                        phase: TouchPhase::Start,
                                        pos,
                                    }));
                                    events.push(BackendEvent::Input(
                                        BackendInputEvent::PointerButton { pos, pressed: true },
                                    ));
                                    InputStatus::Handled
                                }
                                (MotionAction::Up, _)
                                | (MotionAction::PointerUp, _)
                                | (MotionAction::Cancel, _) => {
                                    events.push(BackendEvent::Input(BackendInputEvent::Touch {
                                        phase: TouchPhase::End,
                                        pos: egui::Pos2::ZERO,
                                    }));
                                    events.push(BackendEvent::Input(
                                        BackendInputEvent::PointerButton {
                                            pos: egui::Pos2::ZERO,
                                            pressed: false,
                                        },
                                    ));
                                    InputStatus::Handled
                                }
                                (MotionAction::Move, Some(p)) => {
                                    let pos = egui::pos2(p.x() / pp, p.y() / pp);
                                    events.push(BackendEvent::Input(BackendInputEvent::Touch {
                                        phase: TouchPhase::Move,
                                        pos,
                                    }));
                                    InputStatus::Handled
                                }
                                _ => InputStatus::Unhandled,
                            }
                        }
                        InputEvent::KeyEvent(key) => {
                            if key.action() == KeyAction::Down
                                && key.key_code() == android_activity::input::Keycode::Back
                            {
                                events.push(BackendEvent::Input(BackendInputEvent::Key {
                                    key_code: u32::from(key.key_code()) as i32,
                                    action: BackendKeyAction::Down,
                                }));
                                InputStatus::Handled
                            } else {
                                InputStatus::Unhandled
                            }
                        }
                        _ => InputStatus::Unhandled,
                    }
                });
                if !has {
                    break;
                }
            }
        }
    }
}

impl AndroidBackend for GlBackend {
    fn init(&mut self) -> Result<(), String> {
        log::info!("GlBackend: инициализация");

        if let Some(nw) = self.app.native_window() {
            let egl_state = EglState::create(&nw)?;
            let pp = crate::insets::get_pp(&self.app.config(), nw.width(), nw.height());
            self.dpi = pp;
            self.egl = Some(egl_state);
            Ok(())
        } else {
            Err("GlBackend: нет NativeWindow".into())
        }
    }

    fn poll_events(&mut self) -> Vec<BackendEvent> {
        self.events.clear();
        self.drain_lifecycle_events();
        self.drain_input_events();
        std::mem::take(&mut self.events)
    }

    fn surface_handle(&self) -> SurfaceHandle {
        SurfaceHandle::EglSurface(
            self.egl
                .as_ref()
                .map(|e| e.surface as *mut std::ffi::c_void)
                .unwrap_or(std::ptr::null_mut()),
        )
    }

    fn insets(&self) -> Insets {
        self.insets
    }

    fn dpi(&self) -> f32 {
        self.dpi
    }

    fn show_keyboard(&mut self) {
        log::info!("GlBackend: показать клавиатуру (IME)");
        let vm_ptr = self.app.vm_as_ptr() as usize;
        let activity_ptr = self.app.activity_as_ptr() as usize;
        self.app.run_on_java_main_thread(Box::new(move || {
            show_keyboard_jni(
                vm_ptr as *mut std::ffi::c_void,
                activity_ptr as *mut std::ffi::c_void,
            );
        }));
    }

    fn hide_keyboard(&mut self) {
        log::info!("GlBackend: скрыть клавиатуру (IME)");
        let vm_ptr = self.app.vm_as_ptr() as usize;
        let activity_ptr = self.app.activity_as_ptr() as usize;
        self.app.run_on_java_main_thread(Box::new(move || {
            hide_keyboard_jni(
                vm_ptr as *mut std::ffi::c_void,
                activity_ptr as *mut std::ffi::c_void,
            );
        }));
    }

    fn should_close(&self) -> bool {
        self.should_close.load(Ordering::Relaxed)
    }

    fn egl_state(&self) -> Option<&EglState> {
        self.egl.as_ref()
    }

    fn egl_state_mut(&mut self) -> Option<&mut EglState> {
        self.egl.as_mut()
    }

    fn window_size(&self) -> (u32, u32) {
        self.app
            .native_window()
            .map(|nw| (nw.width() as u32, nw.height() as u32))
            .unwrap_or((0, 0))
    }

    fn vm_ptr(&self) -> *mut std::ffi::c_void {
        self.app.vm_as_ptr()
    }

    fn activity_ptr(&self) -> *mut std::ffi::c_void {
        self.app.activity_as_ptr()
    }

    fn content_rect(&self) -> (i32, i32, i32, i32) {
        let rect = self.app.content_rect();
        (rect.left, rect.top, rect.right, rect.bottom)
    }

    fn recreate_surface(&mut self) -> Result<(), String> {
        if let (Some(ref mut egl), Some(nw)) = (self.egl.as_mut(), self.app.native_window()) {
            egl.recreate_surface(&nw)?;
            log::info!("GlBackend: EGL surface пересоздан");
            Ok(())
        } else {
            Err("GlBackend: нет EGL или NativeWindow для пересоздания surface".into())
        }
    }
}

// ─── IME через JNI ──────────────────────────────────────────────────────
// show_keyboard/hide_keyboard используют run_on_java_main_thread,
// который выполняется на Java main thread. Указатели vm_as_ptr()
// и activity_as_ptr() захватываются до перемещения app в backend
// (в run.rs) или через замыкание (здесь).

/// Показать клавиатуру через JNI.
fn show_keyboard_jni(vm_ptr: *mut std::ffi::c_void, activity_ptr: *mut std::ffi::c_void) {
    use jni::objects::{JObject, JValue};

    if vm_ptr.is_null() || activity_ptr.is_null() {
        log::warn!("show_keyboard: vm_ptr или activity_ptr null");
        return;
    }

    unsafe {
        let jvm = match jni::JavaVM::from_raw(vm_ptr as *mut jni::sys::JavaVM) {
            Ok(jvm) => jvm,
            Err(e) => {
                log::warn!("show_keyboard: JavaVM::from_raw ошибка: {:?}", e);
                return;
            }
        };
        let mut env = match jvm.attach_current_thread() {
            Ok(env) => env,
            Err(e) => {
                log::warn!("show_keyboard: attach_current_thread ошибка: {:?}", e);
                return;
            }
        };

        let activity = JObject::from_raw(activity_ptr as jni::sys::jobject);

        let context = match env
            .call_method(&activity, "getApplicationContext", "()Landroid/content/Context;", &[])
            .ok()
            .and_then(|v| v.l().ok())
        {
            Some(c) => c,
            None => {
                log::warn!("show_keyboard: не удалось получить контекст");
                return;
            }
        };

        let imm_service = env.new_string("input_method").unwrap();
        let imm = match env
            .call_method(
                &context,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[JValue::Object(&imm_service.into())],
            )
            .ok()
            .and_then(|v| v.l().ok())
        {
            Some(imm) => imm,
            None => {
                log::warn!("show_keyboard: не удалось получить InputMethodManager");
                return;
            }
        };

        let window = match env
            .call_method(&activity, "getWindow", "()Landroid/view/Window;", &[])
            .ok()
            .and_then(|v| v.l().ok())
        {
            Some(w) => w,
            None => {
                log::warn!("show_keyboard: не удалось получить Window");
                return;
            }
        };

        let decor_view = match env
            .call_method(&window, "getDecorView", "()Landroid/view/View;", &[])
            .ok()
            .and_then(|v| v.l().ok())
        {
            Some(v) => v,
            None => {
                log::warn!("show_keyboard: не удалось получить DecorView");
                return;
            }
        };

        let _ = env.call_method(
            &imm,
            "showSoftInput",
            "(Landroid/view/View;I)Z",
            &[JValue::Object(&decor_view), JValue::Int(0)],
        );
        log::info!("IME: showSoftInput вызван");
    }
}

/// Скрыть клавиатуру через JNI.
fn hide_keyboard_jni(vm_ptr: *mut std::ffi::c_void, activity_ptr: *mut std::ffi::c_void) {
    use jni::objects::{JObject, JValue};

    if vm_ptr.is_null() || activity_ptr.is_null() {
        log::warn!("hide_keyboard: vm_ptr или activity_ptr null");
        return;
    }

    unsafe {
        let jvm = match jni::JavaVM::from_raw(vm_ptr as *mut jni::sys::JavaVM) {
            Ok(jvm) => jvm,
            Err(e) => {
                log::warn!("hide_keyboard: JavaVM::from_raw ошибка: {:?}", e);
                return;
            }
        };
        let mut env = match jvm.attach_current_thread() {
            Ok(env) => env,
            Err(e) => {
                log::warn!("hide_keyboard: attach_current_thread ошибка: {:?}", e);
                return;
            }
        };

        let activity = JObject::from_raw(activity_ptr as jni::sys::jobject);

        let window = match env
            .call_method(&activity, "getWindow", "()Landroid/view/Window;", &[])
            .ok()
            .and_then(|v| v.l().ok())
        {
            Some(w) => w,
            None => {
                log::warn!("hide_keyboard: не удалось получить Window");
                return;
            }
        };

        let decor_view = match env
            .call_method(&window, "getDecorView", "()Landroid/view/View;", &[])
            .ok()
            .and_then(|v| v.l().ok())
        {
            Some(v) => v,
            None => {
                log::warn!("hide_keyboard: не удалось получить DecorView");
                return;
            }
        };

        let window_token = match env
            .call_method(&decor_view, "getWindowToken", "()Landroid/os/IBinder;", &[])
            .ok()
            .and_then(|v| v.l().ok())
        {
            Some(t) => t,
            None => {
                log::warn!("hide_keyboard: не удалось получить WindowToken");
                return;
            }
        };

        let context = match env
            .call_method(&activity, "getApplicationContext", "()Landroid/content/Context;", &[])
            .ok()
            .and_then(|v| v.l().ok())
        {
            Some(c) => c,
            None => {
                log::warn!("hide_keyboard: не удалось получить контекст");
                return;
            }
        };

        let imm_service = env.new_string("input_method").unwrap();
        let imm = match env
            .call_method(
                &context,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[JValue::Object(&imm_service.into())],
            )
            .ok()
            .and_then(|v| v.l().ok())
        {
            Some(imm) => imm,
            None => {
                log::warn!("hide_keyboard: не удалось получить InputMethodManager");
                return;
            }
        };

        let _ = env.call_method(
            &imm,
            "hideSoftInputFromWindow",
            "(Landroid/os/IBinder;I)Z",
            &[JValue::Object(&window_token), JValue::Int(0)],
        );
        log::info!("IME: hideSoftInputFromWindow вызван");
    }
}
