//! GL-режим backend.
//!
//! Использует GameActivity из AGDK (feature "game-activity").
//! Предоставляет:
//! - IME через show_soft_input / hide_soft_input (без JNI)
//! - Текстовый ввод через text_input_state (без JNI)
//! - SurfaceView для EGL (через native_window)
//! - WindowInsets и DPI из Android SDK (через JNI, т.к. game-activity
//!   пока не пробрасывает их напрямую)
//!
//! # Архитектура
//!
//! `GlBackend` хранит `AndroidApp` и EGL-состояние отдельно от
//! событийного буфера, чтобы избежать конфликтов borrow checker.

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

/// GL-режим backend.
pub struct GlBackend {
    app: AndroidApp,
    egl: Option<EglState>,
    events: Vec<BackendEvent>,
    insets: Insets,
    dpi: f32,
    should_close: AtomicBool,
    ime_visible: bool,
    /// Состояние платформы (insets, theme, clear_color, JNI).
    platform_state: PlatformState,
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
            ime_visible: false,
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
                        log::info!("GlBackend: InitWindow (GameActivity)");
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

    /// Обработать текстовый ввод из IME (GameActivity InputConnection).
    fn drain_text_input(&mut self) {
        // Получаем состояние текстового ввода из GameActivity
        // text_input_state() возвращает текущий текст, selection, compose_region
        let text_state = self.app.text_input_state();

        // Если есть новый текст — отправляем событие
        if !text_state.text.is_empty() {
            log::info!(
                "IME: text='{}' sel={}:{} compose={:?}",
                text_state.text,
                text_state.selection.start,
                text_state.selection.end,
                text_state.compose_region
            );
            self.events.push(BackendEvent::TextInput(text_state.text));
        }
    }
}

impl AndroidBackend for GlBackend {
    fn init(&mut self) -> Result<(), String> {
        log::info!("GlBackend: init — без EGL");
        // Инициализация без EGL — EGL создаётся в init_graphics
        let pp = self
            .app
            .native_window()
            .map(|nw| crate::insets::get_pp(&self.app.config(), nw.width(), nw.height()))
            .unwrap_or(1.0);
        self.dpi = pp;

        Ok(())
    }

    fn init_graphics(&mut self) -> Result<(), BackendError> {
        log::info!("GlBackend: init_graphics (EGL)");
        if let Some(nw) = self.app.native_window() {
            let egl_state = EglState::create(&nw).map_err(|e| BackendError::EglInit(e))?;
            self.egl = Some(egl_state);
            Ok(())
        } else {
            Err(BackendError::NoNativeWindow)
        }
    }

    fn destroy_graphics(&mut self) {
        log::info!("GlBackend: destroy_graphics");
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
        log::info!("GlBackend: показать клавиатуру (IME)");
        self.ime_visible = true;
        self.app.show_soft_input(false);
    }

    fn hide_keyboard(&mut self) {
        log::info!("GlBackend: скрыть клавиатуру (IME)");
        self.ime_visible = false;
        self.app.hide_soft_input(false);
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
            log::info!("GlBackend: EGL surface пересоздан");
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
        // Тема может быть ещё не установлена — используем fallback Light.
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
            SystemBarsStyle::Opaque => {
                // TODO: восстановить непрозрачные бары
            }
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
            // run_on_java_main_thread с пустым замыканием — безопасно,
            // вызывает callback на Java main thread и пробуждает event loop
            let app = unsafe { &*(app_ptr as *const android_activity::AndroidApp) };
            app.run_on_java_main_thread(Box::new(|| {}));
        })
    }
}
