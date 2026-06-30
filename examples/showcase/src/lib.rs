//! Showcase-приложение — точка входа для Android.

#![allow(clippy::new_without_default)]

pub mod app;
pub mod components;
pub mod navigation;
pub mod root_component;
pub mod screens;

#[cfg(target_os = "android")]
use egui_android_framework::android::run;

#[cfg(target_os = "android")]
#[no_mangle]
pub fn android_main(app: android_activity::AndroidApp) {
    run::<app::ShowcaseApplication>(app);
}
