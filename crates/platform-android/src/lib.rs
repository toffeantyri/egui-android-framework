//! Android-платформа — главный цикл, EGL, ввод для egui-приложений.
//!
//! Отвечает за:
//! - Android lifecycle (Resume, Pause, Stop, Destroy)
//! - EGL инициализацию и управление поверхностью
//! - NDK input (касания, клавиатура, кнопка Back)
//! - Главный цикл событий с рендерингом через egui_glow
//! - Backend-абстракцию ([`AndroidBackend`], [`GlBackend`], [`NativeBackend`])
//!
//! Этот крейт НЕ знает про бизнес-логику, State, Reducer, Components.

#![allow(unused_imports, unused_variables, dead_code)]
//!
//! # Сборка APK
//!
//! Для сборки Android-приложения требуется Gradle-проект `android/`
//! с GameActivity (`EguiActivity.kt`), темой и конфигурацией.
//! Он находится в корне репозитория `egui-android-framework/android/`.
//!
//! ## Переменные окружения для Gradle
//!
//! | Переменная | Описание | Пример |
//! |---|---|---|
//! | `APP_LIB_NAME` | Имя `.so` (без префикса lib и .so) | `egui_android_counter` |
//! | `APP_PACKAGE` | `applicationId` | `com.example.myapp` |
//!
//! ## Структура android/
//!
//! ```text
//! android/
//! ├── app/
//! │   ├── src/main/
//! │   │   ├── java/.../EguiActivity.kt   — GameActivity, точка входа
//! │   │   ├── jniLibs/arm64-v8a/         — сюда кладётся .so
//! │   │   └── res/values/styles.xml      — прозрачная тема
//! │   └── build.gradle                   — подхватывает APP_LIB_NAME, APP_PACKAGE
//! ├── build.gradle
//! ├── settings.gradle
//! └── gradle.properties
//! ```
//!
//! ## Пример сборки
//!
//! ```bash
//! # 1. Собрать .so
//! ANDROID_NDK_HOME=/path/to/ndk \
//!   cargo ndk -t arm64-v8a -o android/app/src/main/jniLibs build -p my-app
//!
//! # 2. Собрать APK
//! export APP_LIB_NAME=my_app
//! export APP_PACKAGE=com.example.myapp
//! cd android && ./gradlew assembleDebug && cd ..
//!
//! # 3. Установить и запустить
//! adb install -r android/app/build/outputs/apk/debug/app-debug.apk
//! adb shell am start -n com.example.myapp/com.example.egui_android.EguiActivity
//! ```
//!
//! ## Генератор Android-проекта
//!
//! В крейт входит бинарник `cargo-android-init`, который создаёт `android/`
//! в текущей директории:
//!
//! ```bash
//! APP_PACKAGE=com.example.myapp APP_LIB_NAME=my_app APP_LABEL="My App" \
//!   cargo run --bin cargo-android-init
//! ```
//!
//! После генерации остаётся только собрать `.so` и запустить сборку APK.
//!
//! # Безопасность
//!
//! EGL и OpenGL вызовы — unsafe. Весь unsafe-код изолирован в `egl_backend`.
//! IME JNI-вызовы — unsafe, изолированы в backend/gl_backend.rs.

pub mod backend;
pub mod egl_backend;
pub mod event;
pub mod input;
pub mod insets;
pub mod lifecycle;
pub mod platform_state;
pub mod run;
pub mod system_bars;
pub mod theme;
pub mod waker;

#[cfg(target_os = "android")]
pub use run::run;

// Точка входа для game-activity (используется android-activity glue).
// android_main — функция, которую вызывает GameActivity/NativeActivity glue.
// Она определена в каждом приложении (example/showcase, example/counter).
// Здесь мы только ре-экспортируем функцию run.
