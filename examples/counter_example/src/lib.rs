//! Пример счётчика на реактивной архитектуре (StateStore + UiNotifier).
//!
//! Демонстрирует:
//! - `Component` + чистая View-функция
//! - `Application` как корень DI
//! - Data layer с `StateStore` (без ручного poll())
//! - `UiNotifier` для реактивного обновления UI

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
