//! Состояние платформы — единое хранилище для insets, темы, clear_color и JNI-указателей.
//!
//! Заменяет глобальные статики `Atomic*` и `OnceLock` в `insets.rs`, `theme.rs`, `system_bars.rs`.
//! Хранится только в backend'е. Доступ через `backend.platform_state()`.
//! В `egui::Context` не сохраняется — Application не имеет к нему прямого доступа.
//!
//! # Структура доступна на хосте для тестирования
//!
//! Ядро `PlatformState` (Arc<Mutex>) и `saved_state_buffer` доступны всегда.
//! Android-специфичные методы (JNI, тема, insets через JNI) — только под `target_os = "android"`.

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
    #[cfg(target_os = "android")]
    system_theme: Option<egui_android_platform::SystemTheme>,
    /// Override темы от приложения (None = использовать системную).
    #[cfg(target_os = "android")]
    theme_override: Option<egui_android_platform::SystemTheme>,

    // ─── Clear color ────────────────────────────────────────────
    /// Цвет заливки framebuffer (упакованный 0xRRGGBB).
    clear_color: u32,

    // ─── JNI указатели ──────────────────────────────────────────
    /// Указатель на JavaVM.
    #[cfg(target_os = "android")]
    vm_ptr: *mut std::ffi::c_void,
    /// Указатель на Activity (JNI global reference).
    #[cfg(target_os = "android")]
    activity_ptr: *mut std::ffi::c_void,

    // ─── Saved state buffer ──────────────────────────────────────
    /// Буфер сериализованных данных для JNI-моста kill/restore.
    /// `on_save_state()` → этот буфер → JNI → Bundle →
    /// новый процесс → Bundle → JNI → этот буфер → `on_restore_state()`.
    /// Platform хранит только raw bytes, не знает про ChildStack.
    saved_state_buffer: Option<Vec<u8>>,
}

impl Default for PlatformStateInner {
    fn default() -> Self {
        Self {
            insets_px: InsetsPx::default(),
            insets_valid: false,
            #[cfg(target_os = "android")]
            system_theme: None,
            #[cfg(target_os = "android")]
            theme_override: None,
            clear_color: 0x33_33_33,
            #[cfg(target_os = "android")]
            vm_ptr: std::ptr::null_mut(),
            #[cfg(target_os = "android")]
            activity_ptr: std::ptr::null_mut(),
            saved_state_buffer: None,
        }
    }
}

/// Состояние платформы — thread-safe обёртка над Arc<Mutex>.
///
/// Клонируется дёшево (Arc), все клоны разделяют одно состояние.
/// Хранится только в backend'е.
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
    #[cfg(target_os = "android")]
    pub fn set_system_theme(&self, theme: egui_android_platform::SystemTheme) {
        self.inner.lock().unwrap().system_theme = Some(theme);
    }

    /// Установить override темы от приложения.
    #[cfg(target_os = "android")]
    pub fn set_theme_override(&self, theme: Option<egui_android_platform::SystemTheme>) {
        self.inner.lock().unwrap().theme_override = theme;
    }

    /// Получить текущую тему: override, если установлен, иначе системную.
    #[cfg(target_os = "android")]
    pub fn current_theme(&self) -> egui_android_platform::SystemTheme {
        let inner = self.inner.lock().unwrap();
        if let Some(t) = inner.theme_override {
            return t;
        }
        inner.system_theme.expect("set_system_theme не был вызван")
    }

    /// Получить текущую тему с fallback, если тема ещё не установлена.
    #[cfg(target_os = "android")]
    pub fn current_theme_or_fallback(
        &self,
        fallback: egui_android_platform::SystemTheme,
    ) -> egui_android_platform::SystemTheme {
        let inner = self.inner.lock().unwrap();
        if let Some(t) = inner.theme_override {
            return t;
        }
        inner.system_theme.unwrap_or(fallback)
    }

    // ─── Clear color ────────────────────────────────────────────

    /// Установить clear color (упакованный 0xRRGGBB).
    pub fn set_clear_color(&self, packed: u32) {
        self.inner.lock().unwrap().clear_color = packed;
    }

    /// Установить clear color из Color32.
    #[cfg(target_os = "android")]
    pub fn set_clear_color_from(&self, color: egui::Color32) {
        let packed = ((color.r() as u32) << 16) | ((color.g() as u32) << 8) | (color.b() as u32);
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
    #[cfg(target_os = "android")]
    pub fn set_jni_pointers(&self, vm: *mut std::ffi::c_void, activity: *mut std::ffi::c_void) {
        let mut inner = self.inner.lock().unwrap();
        inner.vm_ptr = vm;
        inner.activity_ptr = activity;
    }

    /// Получить указатель на JavaVM.
    #[cfg(target_os = "android")]
    pub fn vm_ptr(&self) -> *mut std::ffi::c_void {
        self.inner.lock().unwrap().vm_ptr
    }

    /// Получить указатель на Activity.
    #[cfg(target_os = "android")]
    pub fn activity_ptr(&self) -> *mut std::ffi::c_void {
        self.inner.lock().unwrap().activity_ptr
    }

    // ─── Saved state buffer ───────────────────────────────────────

    /// Сохранить сериализованные данные в буфер.
    ///
    /// Вызывается из lifecycle при Stop/Destroy — кладёт результат
    /// `on_save_state()` для передачи в Android Bundle через JNI.
    pub fn set_saved_state(&self, bytes: Vec<u8>) {
        let mut inner = self.inner.lock().unwrap();
        log::info!(
            "PlatformState: set_saved_state — {} байт в буфере",
            bytes.len()
        );
        inner.saved_state_buffer = Some(bytes);
    }

    /// Забрать сериализованные данные из буфера (очищает).
    ///
    /// Вызывается JNI-функцией `nativeGetSavedState` для передачи в Bundle,
    /// и lifecycle при InitWindow для восстановления.
    pub fn take_saved_state(&self) -> Option<Vec<u8>> {
        let mut inner = self.inner.lock().unwrap();
        let result = inner.saved_state_buffer.take();
        if result.is_some() {
            log::info!("PlatformState: take_saved_state — буфер очищен");
        }
        result
    }
}

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

// ─── Тесты ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // В тестах на хосте log не настроен — используем простой логгер
    // или полагаемся на `log` crate с `#[cfg(test)]`.

    #[test]
    fn set_saved_state_and_take_roundtrip() {
        let ps = PlatformState::new();

        // Изначально буфер пуст
        assert!(
            ps.take_saved_state().is_none(),
            "Новый PlatformState: буфер должен быть пуст"
        );

        // Сохраняем данные
        let data = vec![1, 2, 3, 4, 5];
        ps.set_saved_state(data.clone());

        // take возвращает данные и очищает буфер
        let taken = ps.take_saved_state();
        assert_eq!(
            taken,
            Some(data),
            "take_saved_state должен вернуть сохранённые данные"
        );

        // После take буфер снова пуст
        assert!(
            ps.take_saved_state().is_none(),
            "После take_saved_state буфер должен быть очищен"
        );
    }

    #[test]
    fn set_saved_state_overwrites_previous() {
        let ps = PlatformState::new();

        // Первое сохранение
        ps.set_saved_state(vec![10, 20]);
        // Второе сохранение перезаписывает
        ps.set_saved_state(vec![30, 40, 50]);

        let taken = ps.take_saved_state();
        assert_eq!(
            taken,
            Some(vec![30, 40, 50]),
            "set_saved_state должен перезаписывать предыдущее значение"
        );
    }

    #[test]
    fn set_saved_state_empty_vec() {
        let ps = PlatformState::new();

        // Пустой Vec — валидное состояние (стек из 0 элементов)
        ps.set_saved_state(vec![]);

        let taken = ps.take_saved_state();
        assert_eq!(
            taken,
            Some(vec![]),
            "Пустой Vec — валидное сохранённое состояние"
        );
        assert!(ps.take_saved_state().is_none(), "После take буфер очищен");
    }

    #[test]
    fn take_saved_state_idempotent() {
        let ps = PlatformState::new();

        // Многократный take на пустом буфере всегда возвращает None
        assert!(ps.take_saved_state().is_none());
        assert!(ps.take_saved_state().is_none());
        assert!(ps.take_saved_state().is_none());
    }

    #[test]
    fn set_take_set_take_cycle() {
        let ps = PlatformState::new();

        // Цикл 1
        ps.set_saved_state(vec![1]);
        assert_eq!(ps.take_saved_state(), Some(vec![1]));
        assert!(ps.take_saved_state().is_none());

        // Цикл 2 — другие данные
        ps.set_saved_state(vec![9, 8, 7]);
        assert_eq!(ps.take_saved_state(), Some(vec![9, 8, 7]));
        assert!(ps.take_saved_state().is_none());

        // Цикл 3 — ещё раз
        ps.set_saved_state(vec![42]);
        assert_eq!(ps.take_saved_state(), Some(vec![42]));
        assert!(ps.take_saved_state().is_none());
    }

    #[test]
    fn platform_state_clone_shares_buffer() {
        let ps1 = PlatformState::new();
        let ps2 = ps1.clone();

        // Сохраняем через ps1
        ps1.set_saved_state(vec![7, 7, 7]);

        // Читаем через ps2 — тот же буфер (Arc)
        assert_eq!(
            ps2.take_saved_state(),
            Some(vec![7, 7, 7]),
            "Клон PlatformState разделяет буфер через Arc"
        );

        // После take через ps2 — ps1 тоже пуст
        assert!(
            ps1.take_saved_state().is_none(),
            "После take через клон, оригинал тоже пуст"
        );
    }

    #[test]
    fn saved_state_thread_safety() {
        use std::thread;

        let ps = PlatformState::new();
        let ps_clone = ps.clone();

        // Пишем из одного потока
        let handle = thread::spawn(move || {
            ps_clone.set_saved_state(vec![1, 2, 3]);
        });
        handle.join().unwrap();

        // Читаем из другого (текущего)
        assert_eq!(
            ps.take_saved_state(),
            Some(vec![1, 2, 3]),
            "Данные, записанные из другого потока, должны быть видны"
        );
    }
}
