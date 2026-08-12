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
pub mod ime_tests;
pub mod navigation;
pub mod navigation_host;
pub mod screens;

#[cfg(target_os = "android")]
use egui_android_framework::platform_android::run;

#[cfg(target_os = "android")]
#[no_mangle]
pub fn android_main(app: android_activity::AndroidApp) {
    #[cfg(feature = "run-tests")]
    {
        use egui_android_framework::prelude::Application;
        let app_instance = app::ShowcaseApplication::create();
        android_logger::init_once(
            android_logger::Config::default()
                .with_tag(app_instance.config().log_tag.as_str())
                .with_max_level(log::LevelFilter::Info),
        );
        log::info!("=== Режим ТОЛЬКО тесты ===");
        crate::ime_tests::run_ime_tests();
        log::info!("=== Тесты завершены, выход ===");
        // НЕ запускаем приложение — только тесты.
        return;
    }
    run::<app::ShowcaseApplication>(app);
}
