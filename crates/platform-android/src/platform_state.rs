//! Состояние платформы — единое хранилище для insets, темы, clear_color и JNI-указателей.
//!
//! Заменяет глобальные статики `Atomic*` и `OnceLock` в `insets.rs`, `theme.rs`, `system_bars.rs`.
//! Хранится в backend'е и синхронизируется с egui::Context для доступа из Application.

#![cfg(target_os = "android")]

use egui_android_platform::SystemTheme;
use std::sync::{Arc, Mutex};

/// Внутреннее состояние платформы.
#[derive(Debug)]
struct PlatformStateInner {
    // ─── Insets (WindowInsets) ──────────────────────────────────
    /// Системные отступы в пикселях.
    insets_px: InsetsPx,
    /// Флаг: insets были получены через JNI.
    insets_valid: bool,

    // ─── Тема ──────────────────────────────────────────────────
    /// Системная тема (Light/Dark).
    system_theme: Option<SystemTheme>,
    /// Override темы от приложения (None = использовать системную).
    theme_override: Option<SystemTheme>,

    // ─── Clear color ────────────────────────────────────────────
    /// Цвет заливки framebuffer (упакованный 0xRRGGBB).
    clear_color: u32,

    // ─── JNI указатели ──────────────────────────────────────────
    /// Указатель на JavaVM.
    vm_ptr: *mut std::ffi::c_void,
    /// Указатель на Activity (JNI global reference).
    activity_ptr: *mut std::ffi::c_void,
}

impl Default for PlatformStateInner {
    fn default() -> Self {
        Self {
            insets_px: InsetsPx::default(),
            insets_valid: false,
            system_theme: None,
            theme_override: None,
            clear_color: 0x33_33_33,
            vm_ptr: std::ptr::null_mut(),
            activity_ptr: std::ptr::null_mut(),
        }
    }
}

/// Состояние платформы — thread-safe обёртка над Arc<Mutex>.
///
/// Клонируется дёшево (Arc), все клоны разделяют одно состояние.
/// Может храниться в egui::Context::data() для доступа из Application.
#[derive(Debug, Clone)]
pub struct PlatformState {
    inner: Arc<Mutex<PlatformStateInner>>,
}

impl Default for PlatformState {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(PlatformStateInner::default())),
        }
    }
}

impl PlatformState {
    /// Создать новое состояние платформы.
    pub fn new() -> Self {
        Self::default()
    }

    // ─── Insets ─────────────────────────────────────────────────

    /// Установить insets в пикселях (из JNI).
    pub fn set_insets_px(&self, insets: InsetsPx) {
        let mut inner = self.inner.lock().unwrap();
        inner.insets_px = insets;
        inner.insets_valid = true;
    }

    /// Получить insets в пикселях.
    pub fn insets_px(&self) -> InsetsPx {
        self.inner.lock().unwrap().insets_px
    }

    /// Insets были получены через JNI (валидны).
    pub fn insets_are_valid(&self) -> bool {
        let inner = self.inner.lock().unwrap();
        inner.insets_valid && inner.insets_px.is_valid()
    }

    // ─── Тема ───────────────────────────────────────────────────

    /// Установить системную тему (вызывается один раз при старте).
    pub fn set_system_theme(&self, theme: SystemTheme) {
        self.inner.lock().unwrap().system_theme = Some(theme);
    }

    /// Установить override темы от приложения.
    pub fn set_theme_override(&self, theme: Option<SystemTheme>) {
        self.inner.lock().unwrap().theme_override = theme;
    }

    /// Получить текущую тему: override, если установлен, иначе системную.
    pub fn current_theme(&self) -> SystemTheme {
        let inner = self.inner.lock().unwrap();
        if let Some(t) = inner.theme_override {
            return t;
        }
        inner.system_theme.expect("set_system_theme не был вызван")
    }

    // ─── Clear color ────────────────────────────────────────────

    /// Установить clear color (упакованный 0xRRGGBB).
    pub fn set_clear_color(&self, packed: u32) {
        self.inner.lock().unwrap().clear_color = packed;
    }

    /// Установить clear color из Color32.
    pub fn set_clear_color_from(&self, color: egui::Color32) {
        let packed = ((color.r() as u32) << 16)
            | ((color.g() as u32) << 8)
            | (color.b() as u32);
        self.set_clear_color(packed);
    }

    /// Получить текущий clear color как (r, g, b) в диапазоне 0.0..1.0.
    pub fn current_clear_color(&self) -> (f32, f32, f32) {
        let packed = self.inner.lock().unwrap().clear_color;
        let r = ((packed >> 16) & 0xFF) as f32 / 255.0;
        let g = ((packed >> 8) & 0xFF) as f32 / 255.0;
        let b = (packed & 0xFF) as f32 / 255.0;
        (r, g, b)
    }

    // ─── JNI указатели ──────────────────────────────────────────

    /// Сохранить указатели на JavaVM и Activity.
    pub fn set_jni_pointers(&self, vm: *mut std::ffi::c_void, activity: *mut std::ffi::c_void) {
        let mut inner = self.inner.lock().unwrap();
        inner.vm_ptr = vm;
        inner.activity_ptr = activity;
    }

    /// Получить указатель на JavaVM.
    pub fn vm_ptr(&self) -> *mut std::ffi::c_void {
        self.inner.lock().unwrap().vm_ptr
    }

    /// Получить указатель на Activity.
    pub fn activity_ptr(&self) -> *mut std::ffi::c_void {
        self.inner.lock().unwrap().activity_ptr
    }

    // ─── Хранение в egui::Context ─────────────────────────────────

    /// Ключ для хранения PlatformState в egui::Context::data().
    fn cx_key() -> egui::Id {
        egui::Id::new("platform_state")
    }

    /// Сохранить PlatformState в egui::Context.
    pub fn store_in_ctx(&self, ctx: &egui::Context) {
        ctx.data_mut(|data| {
            data.insert_temp(Self::cx_key(), self.clone());
        });
    }

    /// Получить PlatformState из egui::Context.
    pub fn from_ctx(ctx: &egui::Context) -> Option<Self> {
        ctx.data(|data| data.get_temp::<PlatformState>(Self::cx_key()))
    }
}

// PlatformState хранится в egui::Context::data(), а IdTypeMap требует Clone + Send + Sync
// Arc<Mutex<...>> выполняет все требования
unsafe impl Send for PlatformState {}
unsafe impl Sync for PlatformState {}

// ─── InsetsPx ──────────────────────────────────────────────────────

/// Insets в пикселях.
#[derive(Debug, Clone, Copy, Default)]
pub struct InsetsPx {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl InsetsPx {
    /// Все значения ≥ 0 (установлены через JNI).
    pub fn is_valid(&self) -> bool {
        self.left >= 0 && self.top >= 0 && self.right >= 0 && self.bottom >= 0
    }
}
