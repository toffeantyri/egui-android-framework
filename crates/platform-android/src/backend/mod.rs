//! Backend-модуль: абстракция над различными Android backend'ами.
//!
//! Содержит:
//! - [`AndroidBackend`] — трейт для всех backend'ов
//! - [`AndroidBackendKind`] — перечисление типов backend'ов
//! - [`GlBackend`] — GL-режим с IME-поддержкой через JNI
//! - [`NativeBackend`] — NativeActivity режим (fallback)
//!
//! Типы событий определены в [`crate::event`]:
//! - [`BackendEvent`](crate::event::BackendEvent)
//! - [`LifecycleEvent`](crate::event::LifecycleEvent)
//! - [`InputEvent`](crate::event::InputEvent)
//! - [`Insets`](crate::event::Insets)
//! - [`BackendError`](crate::event::BackendError)
//!
//! # Архитектура
//!
//! Каждый backend реализует [`AndroidBackend`] и возвращает [`BackendEvent`].
//! Runtime (run.rs) обрабатывает эти события и конвертирует их в egui-события.
//!
//! # Безопасность
//!
//! Весь unsafe-код изолирован в реализации backend'ов и JNI-вызовах.

#![cfg(target_os = "android")]

#[cfg(target_os = "android")]
mod gl_backend;
#[cfg(target_os = "android")]
mod native_backend;

use crate::event::{BackendError, BackendEvent, InputEvent, Insets, LifecycleEvent, TouchPhase};
use crate::platform_state::PlatformState;
use crate::waker::Waker;
use egui_android_platform::SystemTheme;

/// Стиль системных баров.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemBarsStyle {
    /// Прозрачные бары — контент рисуется под ними.
    Transparent,
    /// Стандартные непрозрачные бары.
    Opaque,
}

/// Поверхность (surface) для рендеринга.
#[derive(Debug, Clone)]
pub enum SurfaceHandle {
    /// EGL поверхность.
    EglSurface(*mut std::ffi::c_void),
    /// Native window (ANativeWindow).
    NativeWindow(*mut std::ffi::c_void),
}

impl SurfaceHandle {
    /// Получить EGL surface как сырой указатель.
    pub fn as_egl_surface(&self) -> *mut std::ffi::c_void {
        match self {
            SurfaceHandle::EglSurface(ptr) => *ptr,
            SurfaceHandle::NativeWindow(ptr) => *ptr,
        }
    }
}

// SAFETY: SurfaceHandle содержит сырые указатели, но они не используются
// в многопоточном контексте — всё в главном потоке.
unsafe impl Send for SurfaceHandle {}
unsafe impl Sync for SurfaceHandle {}

/// Базовый трейт для Android backend'ов.
///
/// Каждый backend (Native, GL, Game) реализует этот трейт.
pub trait AndroidBackend {
    /// Инициализировать backend (без EGL).
    fn init(&mut self) -> Result<(), String>;

    /// Получить накопившиеся события.
    fn poll_events(&mut self) -> Vec<BackendEvent>;

    /// Инициализировать графику (EGL display + context + surface).
    fn init_graphics(&mut self) -> Result<(), BackendError>;

    /// Уничтожить графику (EGL surface + context + display).
    fn destroy_graphics(&mut self);

    /// Получить обработчик поверхности.
    fn surface_handle(&self) -> SurfaceHandle;

    /// Получить текущие WindowInsets.
    fn insets(&self) -> Insets;

    /// Получить текущий DPI (pixels per point).
    fn dpi(&self) -> f32;

    /// Показать клавиатуру (IME).
    fn show_keyboard(&mut self);

    /// Скрыть клавиатуру (IME).
    fn hide_keyboard(&mut self);

    /// Запрошено ли завершение приложения.
    fn should_close(&self) -> bool;

    /// Получить размер окна (ширина, высота).
    fn window_size(&self) -> (u32, u32);

    /// Получить content rect (область содержимого без системных баров).
    fn content_rect(&self) -> (i32, i32, i32, i32);

    /// Пересоздать EGL surface при новом NativeWindow (после Pause/Resume).
    fn recreate_surface(&mut self) -> Result<(), BackendError>;

    // ─── PlatformState (insets, theme, clear_color, JNI) ──────────

    /// Получить доступ к PlatformState.
    fn platform_state(&self) -> &PlatformState;

    /// Получить системные отступы в точках.
    fn system_insets(&self) -> Insets;

    /// Обновить системные отступы через JNI (вызывается при InitWindow).
    fn update_system_insets(&mut self);

    /// Получить текущую тему (с учётом override).
    fn system_theme(&self) -> SystemTheme;

    /// Установить override темы.
    fn set_theme_override(&mut self, theme: Option<SystemTheme>);

    /// Настроить стиль системных баров.
    fn set_system_bars_style(&mut self, style: SystemBarsStyle);

    /// Обменять буферы (EGL swap buffers).
    /// Вызывается после каждого кадра рендеринга.
    fn swap_buffers(&mut self) -> Result<(), String>;

    /// Создать waker для пробуждения event loop.
    ///
    /// Платформенный механизм — backend владеет `AndroidApp` и может
    /// отправлять сигнал Wake в главный цикл.
    fn create_waker(&self) -> Waker;
}

/// Тип Android backend'а.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AndroidBackendKind {
    /// NativeActivity backend (ANativeWindow + EGL) — fallback.
    Native,
    /// GL-режим с IME-поддержкой (основной).
    Gl,
    /// GameActivity backend (будущий).
    Game,
}

#[cfg(target_os = "android")]
pub use gl_backend::GlBackend;
#[cfg(target_os = "android")]
pub use native_backend::NativeBackend;

#[cfg(target_os = "android")]
impl AndroidBackend for Box<dyn AndroidBackend> {
    fn init(&mut self) -> Result<(), String> {
        (**self).init()
    }

    fn poll_events(&mut self) -> Vec<BackendEvent> {
        (**self).poll_events()
    }

    fn init_graphics(&mut self) -> Result<(), BackendError> {
        (**self).init_graphics()
    }

    fn destroy_graphics(&mut self) {
        (**self).destroy_graphics()
    }

    fn surface_handle(&self) -> SurfaceHandle {
        (**self).surface_handle()
    }

    fn insets(&self) -> Insets {
        (**self).insets()
    }

    fn dpi(&self) -> f32 {
        (**self).dpi()
    }

    fn show_keyboard(&mut self) {
        (**self).show_keyboard()
    }

    fn hide_keyboard(&mut self) {
        (**self).hide_keyboard()
    }

    fn should_close(&self) -> bool {
        (**self).should_close()
    }

    fn window_size(&self) -> (u32, u32) {
        (**self).window_size()
    }

    fn content_rect(&self) -> (i32, i32, i32, i32) {
        (**self).content_rect()
    }

    fn recreate_surface(&mut self) -> Result<(), BackendError> {
        (**self).recreate_surface()
    }

    fn platform_state(&self) -> &PlatformState {
        (**self).platform_state()
    }

    fn system_insets(&self) -> Insets {
        (**self).system_insets()
    }

    fn update_system_insets(&mut self) {
        (**self).update_system_insets()
    }

    fn system_theme(&self) -> SystemTheme {
        (**self).system_theme()
    }

    fn set_theme_override(&mut self, theme: Option<SystemTheme>) {
        (**self).set_theme_override(theme)
    }

    fn set_system_bars_style(&mut self, style: SystemBarsStyle) {
        (**self).set_system_bars_style(style)
    }

    fn swap_buffers(&mut self) -> Result<(), String> {
        (**self).swap_buffers()
    }

    fn create_waker(&self) -> Waker {
        (**self).create_waker()
    }
}
