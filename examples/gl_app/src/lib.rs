//! Пример GL-приложения на GameActivity.
//!
//! Сборка:
//!   ANDROID_NDK_HOME=/path/to/ndk cargo ndk -t arm64-v8a -o ../../android/app/src/main/jniLibs build -p egui-gl-app
//!
//! Запуск:
//!   cd android && ./gradlew assembleDebug && adb install -r app/build/outputs/apk/debug/app-debug.apk

#![cfg(target_os = "android")]

use android_activity::AndroidApp;
use egui_android_platform_android::run;
use egui_android_runtime::{AppConfig, Application};

struct MyApp {
    config: AppConfig,
}

impl Application for MyApp {
    type RootComponent = ();

    fn create() -> Self {
        let config = AppConfig {
            log_tag: String::from("egui-gl-app"),
            target_fps: 60,
        };
        MyApp { config }
    }

    fn root(&mut self) -> &mut () {
        // RootComponent = (), возвращаем ссылку на статический unit
        static mut UNIT: () = ();
        unsafe { &mut UNIT }
    }

    fn root_ref(&self) -> &() {
        static UNIT: () = ();
        &UNIT
    }

    fn config(&self) -> &AppConfig {
        &self.config
    }

    fn config_mut(&mut self) -> &mut AppConfig {
        &mut self.config
    }

    fn on_back_pressed(&mut self) {
        log::info!("Back pressed — завершаем");
    }

    fn request_destroy(&mut self) -> bool {
        false
    }
}

#[no_mangle]
fn android_main(app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_tag("egui-gl-app")
            .with_max_level(log::LevelFilter::Debug),
    );
    log::info!("Запуск GL-приложения через GameActivity");

    run::run::<MyApp>(app);
}
