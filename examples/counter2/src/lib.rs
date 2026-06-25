//! Пример счётчика на Decompose-style архитектуре.
//!
//! Демонстрирует:
//! - `Component` + чистая View-функция
//! - `Application` как корень DI
//! - Data layer в фоновом потоке через mpsc
//! - `poll()` — опрос событий из data layer

pub mod app;
pub mod component;
pub mod data_layer;
pub mod msg;
pub mod view;

#[cfg(target_os = "android")]
use egui_android_framework::android::run;

#[cfg(target_os = "android")]
#[no_mangle]
pub fn android_main(app: android_activity::AndroidApp) {
    run::<app::CounterApp>(app);
}
