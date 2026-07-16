//! Backend-модуль: абстракция над различными Android backend'ами.
//!
//! Содержит:
//! - [`AndroidBackend`] — трейт для всех backend'ов
//! - [`AndroidBackendKind`] — перечисление типов backend'ов
//! - [`GlBackend`] — GL-режим с IME-поддержкой через JNI
//! - [`NativeBackend`] — NativeActivity режим (fallback)
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

#[cfg(target_os = "android")]
use crate::egl_backend::EglState;

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

/// Отступы (WindowInsets) в точках.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Insets {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Default for Insets {
    fn default() -> Self {
        Self {
            left: 0.0,
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
        }
    }
}

/// Событие бэкенда.
///
/// Пробрасывается из backend в Runtime для конвертации в egui-события.
pub enum BackendEvent {
    /// Событие жизненного цикла.
    Lifecycle(LifecycleEvent),
    /// Событие ввода (touch, key).
    Input(InputEvent),
    /// Текстовый ввод от IME.
    TextInput(String),
    /// Изменение WindowInsets.
    InsetsChanged(Insets),
    /// Изменение DPI.
    DpiChanged(f32),
}

/// Событие жизненного цикла.
#[derive(Debug, Clone, Copy)]
pub enum LifecycleEvent {
    /// Окно инициализировано (или пересоздано).
    InitWindow,
    /// Приложение возобновило работу.
    Resume,
    /// Приложение приостановлено.
    Pause,
    /// Приложение остановлено.
    Stop,
    /// Приложение уничтожено.
    Destroy,
}

/// Событие ввода.
pub enum InputEvent {
    /// Сенсорное событие.
    Touch { phase: TouchPhase, pos: egui::Pos2 },
    /// Событие кнопки указателя.
    PointerButton { pos: egui::Pos2, pressed: bool },
    /// Событие клавиши.
    Key { key_code: i32, action: KeyAction },
}

/// Фаза сенсорного события.
#[derive(Debug, Clone, Copy)]
pub enum TouchPhase {
    Start,
    Move,
    End,
    Cancel,
}

/// Действие клавиши.
#[derive(Debug, Clone, Copy)]
pub enum KeyAction {
    Down,
    Up,
}

/// Базовый трейт для Android backend'ов.
///
/// Каждый backend (Native, GL, Game) реализует этот трейт.
pub trait AndroidBackend {
    /// Инициализировать backend.
    fn init(&mut self) -> Result<(), String>;

    /// Получить накопившиеся события.
    fn poll_events(&mut self) -> Vec<BackendEvent>;

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

    /// Получить EGL состояние (для рендеринга).
    fn egl_state(&self) -> Option<&EglState>;

    /// Получить мутабельное EGL состояние.
    fn egl_state_mut(&mut self) -> Option<&mut EglState>;

    /// Получить размер окна (ширина, высота).
    fn window_size(&self) -> (u32, u32);

    /// Получить указатель на JavaVM.
    fn vm_ptr(&self) -> *mut std::ffi::c_void;

    /// Получить указатель на Activity (JNI global reference).
    fn activity_ptr(&self) -> *mut std::ffi::c_void;

    /// Получить content rect (область содержимого без системных баров).
    fn content_rect(&self) -> (i32, i32, i32, i32);

    /// Пересоздать EGL surface при новом NativeWindow (после Pause/Resume).
    fn recreate_surface(&mut self) -> Result<(), String>;
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

    fn egl_state(&self) -> Option<&EglState> {
        (**self).egl_state()
    }

    fn egl_state_mut(&mut self) -> Option<&mut EglState> {
        (**self).egl_state_mut()
    }

    fn window_size(&self) -> (u32, u32) {
        (**self).window_size()
    }

    fn vm_ptr(&self) -> *mut std::ffi::c_void {
        (**self).vm_ptr()
    }

    fn activity_ptr(&self) -> *mut std::ffi::c_void {
        (**self).activity_ptr()
    }

    fn content_rect(&self) -> (i32, i32, i32, i32) {
        (**self).content_rect()
    }

    fn recreate_surface(&mut self) -> Result<(), String> {
        (**self).recreate_surface()
    }
}
