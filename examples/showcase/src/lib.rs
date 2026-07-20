//! Showcase-приложение — точка входа для Android.
//!
//! Сборка:
//!   cd examples/showcase && ./run_android.sh
//!
//! Запуск:
//!   cd examples/showcase && ./run_android.sh --run

#![allow(clippy::new_without_default)]

pub mod app;
pub mod factory;
pub mod navigation;
pub mod navigation_host;
pub mod screens;

#[cfg(target_os = "android")]
use egui_android_framework::platform_android::run;

#[cfg(target_os = "android")]
#[no_mangle]
pub fn android_main(app: android_activity::AndroidApp) {
    run::<app::ShowcaseApplication>(app);
}
