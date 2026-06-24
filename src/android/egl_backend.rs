//! Низкоуровневый EGL backend для Android: константы, EGL bindings и состояние.
//!
//! Содержит:
//! - `pub(crate) mod egl` — FFI-объявления функций libEGL.so
//! - `pub(crate) struct EglState` — создание/уничтожение EGL display, surface, контекста

#![cfg(target_os = "android")]

use ndk::native_window::NativeWindow;
use raw_window_handle::HasRawWindowHandle;

// ---------------------------------------------------------------------------
// EGL bindings — минимум, необходимый для работы
// ---------------------------------------------------------------------------

/// Указатели на стандартные EGL функции из libEGL.so.
/// Android гарантирует наличие EGL 1.4 в libEGL.so.
pub(crate) mod egl {
    #![allow(non_camel_case_types, dead_code)]

    pub type EGLBoolean = u32;
    pub type EGLint = i32;
    pub type EGLDisplay = *mut std::ffi::c_void;
    pub type EGLConfig = *mut std::ffi::c_void;
    pub type EGLSurface = *mut std::ffi::c_void;
    pub type EGLContext = *mut std::ffi::c_void;
    pub type EGLNativeWindowType = *mut std::ffi::c_void;

    pub const EGL_DEFAULT_DISPLAY: EGLDisplay = std::ptr::null_mut();
    pub const EGL_NO_CONTEXT: EGLContext = std::ptr::null_mut();
    pub const EGL_NO_SURFACE: EGLSurface = std::ptr::null_mut();

    pub const EGL_TRUE: EGLBoolean = 1;
    pub const EGL_FALSE: EGLBoolean = 0;

    pub const EGL_SUCCESS: EGLint = 0x3000;
    pub const EGL_NOT_INITIALIZED: EGLint = 0x3001;
    pub const EGL_BAD_ACCESS: EGLint = 0x3002;
    pub const EGL_BAD_ALLOC: EGLint = 0x3003;
    pub const EGL_BAD_ATTRIBUTE: EGLint = 0x3004;
    pub const EGL_BAD_CONFIG: EGLint = 0x3005;
    pub const EGL_BAD_CONTEXT: EGLint = 0x3006;
    pub const EGL_BAD_CURRENT_SURFACE: EGLint = 0x3007;
    pub const EGL_BAD_DISPLAY: EGLint = 0x3008;
    pub const EGL_BAD_MATCH: EGLint = 0x3009;
    pub const EGL_BAD_NATIVE_PIXMAP: EGLint = 0x300A;
    pub const EGL_BAD_NATIVE_WINDOW: EGLint = 0x300B;
    pub const EGL_BAD_PARAMETER: EGLint = 0x300C;
    pub const EGL_BAD_SURFACE: EGLint = 0x300D;
    pub const EGL_CONTEXT_LOST: EGLint = 0x300E;
    pub const EGL_BUFFER_SIZE: EGLint = 0x3020;
    pub const EGL_ALPHA_SIZE: EGLint = 0x3021;
    pub const EGL_BLUE_SIZE: EGLint = 0x3022;
    pub const EGL_GREEN_SIZE: EGLint = 0x3023;
    pub const EGL_RED_SIZE: EGLint = 0x3024;
    pub const EGL_DEPTH_SIZE: EGLint = 0x3025;
    pub const EGL_STENCIL_SIZE: EGLint = 0x3026;
    pub const EGL_SURFACE_TYPE: EGLint = 0x3033;
    pub const EGL_WINDOW_BIT: EGLint = 0x0004;
    pub const EGL_RENDERABLE_TYPE: EGLint = 0x3040;
    pub const EGL_OPENGL_ES2_BIT: EGLint = 0x0004;
    pub const EGL_NONE: EGLint = 0x3038;
    pub const EGL_WIDTH: EGLint = 0x3057;
    pub const EGL_HEIGHT: EGLint = 0x3056;
    pub const EGL_CONTEXT_CLIENT_VERSION: EGLint = 0x3098;
    pub const EGL_OPENGL_ES_API: EGLint = 0x30A0;

    extern "C" {
        pub fn eglGetDisplay(display_id: EGLDisplay) -> EGLDisplay;
        pub fn eglInitialize(dpy: EGLDisplay, major: *mut EGLint, minor: *mut EGLint)
            -> EGLBoolean;
        pub fn eglChooseConfig(
            dpy: EGLDisplay,
            attrib_list: *const EGLint,
            configs: *mut EGLConfig,
            config_size: EGLint,
            num_config: *mut EGLint,
        ) -> EGLBoolean;
        pub fn eglCreateWindowSurface(
            dpy: EGLDisplay,
            config: EGLConfig,
            win: EGLNativeWindowType,
            attrib_list: *const EGLint,
        ) -> EGLSurface;
        pub fn eglCreateContext(
            dpy: EGLDisplay,
            config: EGLConfig,
            share_context: EGLContext,
            attrib_list: *const EGLint,
        ) -> EGLContext;
        pub fn eglMakeCurrent(
            dpy: EGLDisplay,
            draw: EGLSurface,
            read: EGLSurface,
            ctx: EGLContext,
        ) -> EGLBoolean;
        pub fn eglSwapBuffers(dpy: EGLDisplay, surface: EGLSurface) -> EGLBoolean;
        pub fn eglDestroySurface(dpy: EGLDisplay, surface: EGLSurface) -> EGLBoolean;
        pub fn eglDestroyContext(dpy: EGLDisplay, ctx: EGLContext) -> EGLBoolean;
        pub fn eglTerminate(dpy: EGLDisplay) -> EGLBoolean;
        pub fn eglGetError() -> EGLint;
        pub fn eglQuerySurface(
            dpy: EGLDisplay,
            surface: EGLSurface,
            attribute: EGLint,
            value: *mut EGLint,
        ) -> EGLBoolean;
    }

    /// Получить читаемое описание EGL ошибки.
    pub fn egl_error_str(err: EGLint) -> &'static str {
        match err {
            EGL_SUCCESS => "EGL_SUCCESS",
            EGL_NOT_INITIALIZED => "EGL_NOT_INITIALIZED",
            EGL_BAD_ACCESS => "EGL_BAD_ACCESS",
            EGL_BAD_ALLOC => "EGL_BAD_ALLOC",
            EGL_BAD_ATTRIBUTE => "EGL_BAD_ATTRIBUTE",
            EGL_BAD_CONFIG => "EGL_BAD_CONFIG",
            EGL_BAD_CONTEXT => "EGL_BAD_CONTEXT",
            EGL_BAD_CURRENT_SURFACE => "EGL_BAD_CURRENT_SURFACE",
            EGL_BAD_DISPLAY => "EGL_BAD_DISPLAY",
            EGL_BAD_MATCH => "EGL_BAD_MATCH",
            EGL_BAD_NATIVE_PIXMAP => "EGL_BAD_NATIVE_PIXMAP",
            EGL_BAD_NATIVE_WINDOW => "EGL_BAD_NATIVE_WINDOW",
            EGL_BAD_PARAMETER => "EGL_BAD_PARAMETER",
            EGL_BAD_SURFACE => "EGL_BAD_SURFACE",
            EGL_CONTEXT_LOST => "EGL_CONTEXT_LOST",
            _ => "EGL_UNKNOWN",
        }
    }
}

// ---------------------------------------------------------------------------
// EGL состояние
// ---------------------------------------------------------------------------

pub(crate) struct EglState {
    pub(crate) display: egl::EGLDisplay,
    pub(crate) surface: egl::EGLSurface,
    pub(crate) context: egl::EGLContext,
}

impl EglState {
    /// Создать EGL display, surface для данного native window и контекст GLES 2.
    pub(crate) fn create(native_window: &NativeWindow) -> Result<Self, String> {
        let display = unsafe { egl::eglGetDisplay(egl::EGL_DEFAULT_DISPLAY) };
        if display.is_null() {
            return Err("eglGetDisplay returned null".into());
        }

        let mut major: egl::EGLint = 0;
        let mut minor: egl::EGLint = 0;
        if unsafe { egl::eglInitialize(display, &mut major, &mut minor) } == egl::EGL_FALSE {
            let err = unsafe { egl::eglGetError() };
            return Err(format!("eglInitialize failed: {}", egl::egl_error_str(err)));
        }
        log::info!("EGL initialized: {}.{}", major, minor);

        // Выбираем конфиг: RGBA8888 + depth + stencil
        let attrib_list = [
            egl::EGL_SURFACE_TYPE,
            egl::EGL_WINDOW_BIT,
            egl::EGL_RENDERABLE_TYPE,
            egl::EGL_OPENGL_ES2_BIT,
            egl::EGL_RED_SIZE,
            8,
            egl::EGL_GREEN_SIZE,
            8,
            egl::EGL_BLUE_SIZE,
            8,
            egl::EGL_ALPHA_SIZE,
            8,
            egl::EGL_DEPTH_SIZE,
            16,
            egl::EGL_STENCIL_SIZE,
            8,
            egl::EGL_NONE,
        ];

        let mut config: egl::EGLConfig = std::ptr::null_mut();
        let mut num_config: egl::EGLint = 0;
        if unsafe {
            egl::eglChooseConfig(
                display,
                attrib_list.as_ptr(),
                &mut config,
                1,
                &mut num_config,
            )
        } == egl::EGL_FALSE
            || num_config == 0
        {
            let err = unsafe { egl::eglGetError() };
            return Err(format!(
                "eglChooseConfig failed: {}",
                egl::egl_error_str(err)
            ));
        }

        // Создаём window surface
        let win_handle = native_window.raw_window_handle();
        let egl_native_window = match win_handle {
            raw_window_handle::RawWindowHandle::AndroidNdk(handle) => {
                handle.a_native_window.as_ptr() as egl::EGLNativeWindowType
            }
            _ => return Err("Unsupported RawWindowHandle".into()),
        };

        let surface = unsafe {
            egl::eglCreateWindowSurface(display, config, egl_native_window, std::ptr::null())
        };
        if surface.is_null() {
            let err = unsafe { egl::eglGetError() };
            return Err(format!(
                "eglCreateWindowSurface failed: {}",
                egl::egl_error_str(err)
            ));
        }

        // Создаём GLES 2 контекст
        let ctx_attribs = [egl::EGL_CONTEXT_CLIENT_VERSION, 2, egl::EGL_NONE];
        let context = unsafe {
            egl::eglCreateContext(display, config, egl::EGL_NO_CONTEXT, ctx_attribs.as_ptr())
        };
        if context.is_null() {
            let err = unsafe { egl::eglGetError() };
            return Err(format!(
                "eglCreateContext failed: {}",
                egl::egl_error_str(err)
            ));
        }

        // Делаем контекст текущим
        if unsafe { egl::eglMakeCurrent(display, surface, surface, context) } == egl::EGL_FALSE {
            let err = unsafe { egl::eglGetError() };
            return Err(format!(
                "eglMakeCurrent failed: {}",
                egl::egl_error_str(err)
            ));
        }

        Ok(Self {
            display,
            surface,
            context,
        })
    }

    pub(crate) fn destroy(&mut self) {
        unsafe {
            egl::eglMakeCurrent(
                self.display,
                egl::EGL_NO_SURFACE,
                egl::EGL_NO_SURFACE,
                egl::EGL_NO_CONTEXT,
            );
            if !self.surface.is_null() {
                egl::eglDestroySurface(self.display, self.surface);
                self.surface = egl::EGL_NO_SURFACE;
            }
            if !self.context.is_null() {
                egl::eglDestroyContext(self.display, self.context);
                self.context = egl::EGL_NO_CONTEXT;
            }
            if !self.display.is_null() {
                egl::eglTerminate(self.display);
                self.display = egl::EGL_DEFAULT_DISPLAY;
            }
        }
    }

    pub(crate) fn swap_buffers(&self) -> Result<(), String> {
        if unsafe { egl::eglSwapBuffers(self.display, self.surface) } == egl::EGL_FALSE {
            let err = unsafe { egl::eglGetError() };
            return Err(format!(
                "eglSwapBuffers failed: {}",
                egl::egl_error_str(err)
            ));
        }
        Ok(())
    }
}
