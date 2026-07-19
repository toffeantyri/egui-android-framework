//! Состояние темы: системная тема + override от приложения.
//!
//! Глобальные статики отсутствуют — состояние хранится только в backend.
//! Доступ через `backend.platform_state()`.
//!
//! Application задаёт тему через egui::Context (MaterialTheme::light/dark),
//! а platform-android сам применяет clear_color и system_bars.
//!
//! Публичный API больше не предоставляет функции, работающие через
//! `PlatformState::from_ctx()` — PlatformState хранится только в backend.

#![cfg(target_os = "android")]
