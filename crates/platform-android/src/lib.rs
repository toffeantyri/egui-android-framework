//! Android-платформа — реализация [`Platform`] для Android.
//!
//! Отвечает за:
//! - Android lifecycle (Resume, Pause, Stop, Destroy)
//! - EGL инициализацию и управление поверхностью
//! - NDK input (касания, клавиатура, кнопка Back)
//! - Главный цикл событий с рендерингом через egui_glow
//! - Backend-абстракцию (AndroidBackend, GlBackend, NativeBackend)
//!
//! Этот крейт НЕ знает про бизнес-логику, State, Reducer, Components.
//!
//! # Безопасность
//!
//! EGL и OpenGL вызовы — unsafe. Весь unsafe-код изолирован в `egl_backend`.
//! IME JNI-вызовы — unsafe, изолированы в backend/gl_backend.rs.

pub mod backend;
pub mod egl_backend;
pub mod input;
pub mod insets;
pub mod platform_state;
pub mod run;
pub mod system_bars;
pub mod theme;

#[cfg(target_os = "android")]
pub use run::run;

// Точка входа для game-activity (используется android-activity glue).
// android_main — функция, которую вызывает GameActivity/NativeActivity glue.
// Она определена в каждом приложении (example/showcase, example/counter).
// Здесь мы только ре-экспортируем функцию run.
