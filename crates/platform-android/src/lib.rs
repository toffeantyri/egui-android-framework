//! Android-платформа — реализация [`Platform`] для Android.
//!
//! Отвечает за:
//! - Android lifecycle (Resume, Pause, Stop, Destroy)
//! - EGL инициализацию и управление поверхностью
//! - NDK input (касания, клавиатура, кнопка Back)
//! - Главный цикл событий с рендерингом через egui_glow
//!
//! Этот крейт НЕ знает про бизнес-логику, State, Reducer, Components.
//!
//! # Безопасность
//!
//! EGL и OpenGL вызовы — unsafe. Весь unsafe-код изолирован в `egl_backend`.
