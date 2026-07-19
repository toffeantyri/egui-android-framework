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

use crate::backend::{AndroidBackend, SurfaceHandle, SystemBarsStyle};
use crate::egl_backend::EglState;
use crate::event::{
    BackendError, BackendEvent, InputEvent as BackendInputEvent, Insets,
    KeyAction as BackendKeyAction, LifecycleEvent, TouchPhase,
};
use crate::platform_state::PlatformState;
use crate::waker::Waker;
use egui_android_platform::SystemTheme;

/// NativeActivity backend — fallback.
pub struct NativeBackend {
    app: AndroidApp,
    egl: Option<EglState>,
    events: Vec<BackendEvent>,
    insets: Insets,
    dpi: f32,
    should_close: AtomicBool,
    /// Состояние платформы (insets, theme, clear_color, JNI).
    platform_state: PlatformState,
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
            platform_state: PlatformState::new(),
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
        log::info!("NativeBackend: init — без EGL");
        let pp = self
            .app
            .native_window()
            .map(|nw| crate::insets::get_pp(&self.app.config(), nw.width(), nw.height()))
            .unwrap_or(1.0);
        self.dpi = pp;

        Ok(())
    }

    fn init_graphics(&mut self) -> Result<(), BackendError> {
        log::info!("NativeBackend: init_graphics (EGL)");
        if let Some(nw) = self.app.native_window() {
            let egl_state = EglState::create(&nw).map_err(|e| BackendError::EglInit(e))?;
            self.egl = Some(egl_state);
            Ok(())
        } else {
            Err(BackendError::NoNativeWindow)
        }
    }

    fn destroy_graphics(&mut self) {
        log::info!("NativeBackend: destroy_graphics");
        if let Some(mut egl) = self.egl.take() {
            egl.destroy();
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

    fn window_size(&self) -> (u32, u32) {
        self.app
            .native_window()
            .map(|nw| (nw.width() as u32, nw.height() as u32))
            .unwrap_or((0, 0))
    }

    fn content_rect(&self) -> (i32, i32, i32, i32) {
        let rect = self.app.content_rect();
        (rect.left, rect.top, rect.right, rect.bottom)
    }

    fn recreate_surface(&mut self) -> Result<(), BackendError> {
        if let (Some(ref mut egl), Some(nw)) = (self.egl.as_mut(), self.app.native_window()) {
            egl.recreate_surface(&nw)
                .map_err(|e| BackendError::RecreateSurface(e))?;
            log::info!("NativeBackend: EGL surface пересоздан");
            Ok(())
        } else {
            Err(BackendError::Other(
                "нет EGL или NativeWindow для пересоздания surface".into(),
            ))
        }
    }

    // ─── PlatformState ──────────────────────────────────────────────

    fn platform_state(&self) -> &PlatformState {
        &self.platform_state
    }

    fn system_insets(&self) -> Insets {
        let pp = self.dpi;
        let state = &self.platform_state;
        if state.insets_are_valid() {
            let px = state.insets_px();
            Insets {
                left: px.left as f32 / pp,
                top: px.top as f32 / pp,
                right: px.right as f32 / pp,
                bottom: px.bottom as f32 / pp,
            }
        } else {
            self.insets
        }
    }

    fn update_system_insets(&mut self) {
        let vm = self.app.vm_as_ptr();
        let activity = self.app.activity_as_ptr();
        if vm.is_null() || activity.is_null() {
            return;
        }
        if let Some(insets_px) = crate::insets::get_system_insets_jni(vm, activity) {
            self.platform_state.set_insets_px(insets_px);
        }
    }

    fn system_theme(&self) -> SystemTheme {
        self.platform_state.current_theme()
    }

    fn set_theme_override(&mut self, theme: Option<SystemTheme>) {
        self.platform_state.set_theme_override(theme);
    }

    fn set_system_bars_style(&mut self, style: SystemBarsStyle) {
        let vm = self.app.vm_as_ptr();
        let activity = self.app.activity_as_ptr();
        if vm.is_null() || activity.is_null() {
            return;
        }

        let vm_ptr = vm as usize;
        let activity_ptr = activity as usize;
        let theme = self
            .platform_state
            .current_theme_or_fallback(egui_android_platform::SystemTheme::Light);

        match style {
            SystemBarsStyle::Transparent => {
                self.app.run_on_java_main_thread(Box::new(move || {
                    let vm = vm_ptr as *mut std::ffi::c_void;
                    let activity = activity_ptr as *mut std::ffi::c_void;
                    if !vm.is_null() && !activity.is_null() {
                        crate::system_bars::set_transparent_system_bars(vm, activity);
                        crate::system_bars::apply_system_bars_color_jni(vm, activity, theme);
                    }
                }));
            }
            SystemBarsStyle::Opaque => {}
        }
    }

    fn swap_buffers(&mut self) -> Result<(), String> {
        if let Some(ref egl) = self.egl {
            egl.swap_buffers()
        } else {
            Err("EGL не инициализирован".into())
        }
    }

    fn create_waker(&self) -> Waker {
        let app = self.app();
        let app_ptr = app as *const android_activity::AndroidApp as usize;
        Waker::new(move || {
            let app = unsafe { &*(app_ptr as *const android_activity::AndroidApp) };
            app.run_on_java_main_thread(Box::new(|| {}));
        })
    }
}
