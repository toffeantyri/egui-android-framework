//! NativeActivity backend — fallback-режим.
//!
//! Реализует [`AndroidBackend`] через ANativeWindow и EGL.
//! Используется как fallback, если GL-режим недоступен.
//!
//! Ограничения:
//! - Нет IME (клавиатурный ввод не поддерживается)
//! - WindowInsets только через JNI (без автоматических событий)
//! - Нет текстового ввода (пустой TextInput)

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

/// NativeActivity backend — fallback.
pub struct NativeBackend {
    app: AndroidApp,
    egl: Option<EglState>,
    events: Vec<BackendEvent>,
    insets: Insets,
    dpi: f32,
    should_close: AtomicBool,
}

impl NativeBackend {
    /// Создать новый NativeBackend.
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

        app.poll_events(
            Some(std::time::Duration::from_millis(0)),
            |event| match event {
                android_activity::PollEvent::Wake | android_activity::PollEvent::Timeout => {}
                android_activity::PollEvent::Main(e) => match e {
                    MainEvent::InitWindow { .. } => {
                        log::info!("NativeBackend: InitWindow");
                        events.push(BackendEvent::Lifecycle(LifecycleEvent::InitWindow));
                        if let Some(nw) = app.native_window() {
                            let pp = crate::insets::get_pp(&app.config(), nw.width(), nw.height());
                            events.push(BackendEvent::DpiChanged(pp));
                        }
                    }
                    MainEvent::Resume { .. } => {
                        log::info!("NativeBackend: Resume");
                        events.push(BackendEvent::Lifecycle(LifecycleEvent::Resume));
                    }
                    MainEvent::Pause { .. } => {
                        log::info!("NativeBackend: Pause");
                        events.push(BackendEvent::Lifecycle(LifecycleEvent::Pause));
                    }
                    MainEvent::Stop { .. } => {
                        log::info!("NativeBackend: Stop");
                        events.push(BackendEvent::Lifecycle(LifecycleEvent::Stop));
                    }
                    MainEvent::Destroy { .. } => {
                        log::info!("NativeBackend: Destroy");
                        events.push(BackendEvent::Lifecycle(LifecycleEvent::Destroy));
                    }
                    _ => {}
                },
                _ => {}
            },
        );
    }

    /// Слить input события.
    fn drain_input_events(&mut self) {
        let events = &mut self.events;
        let pp = self.dpi;

        if let Ok(mut iter) = self.app.input_events_iter() {
            loop {
                let has = iter.next(|event| match event {
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
                });
                if !has {
                    break;
                }
            }
        }
    }
}

impl AndroidBackend for NativeBackend {
    fn init(&mut self) -> Result<(), String> {
        log::info!("NativeBackend: инициализация");

        if let Some(nw) = self.app.native_window() {
            let egl_state = EglState::create(&nw)?;

            let pp = crate::insets::get_pp(&self.app.config(), nw.width(), nw.height());
            self.dpi = pp;

            self.egl = Some(egl_state);
            Ok(())
        } else {
            Err("NativeBackend: нет NativeWindow".into())
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
        log::info!("NativeBackend: показать клавиатуру — не поддерживается в NativeActivity");
    }

    fn hide_keyboard(&mut self) {
        log::info!("NativeBackend: скрыть клавиатуру — не поддерживается в NativeActivity");
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
            log::info!("NativeBackend: EGL surface пересоздан");
            Ok(())
        } else {
            Err("NativeBackend: нет EGL или NativeWindow для пересоздания surface".into())
        }
    }
}
