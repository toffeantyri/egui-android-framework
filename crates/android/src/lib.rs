//! cdylib-обёртка для Android-приложения.
//!
//! Собирается как .so, загружается через System.loadLibrary из GameActivity.
//! Точка входа: android_main(app: AndroidApp).
//!
//! Каждое конкретное приложение (showcase, counter) должно определить свою
//! реализацию `Application` и вызывать её из `android_main`.
//! Этот крейт — только обёртка для экспорта символа android_main.

// android_main — экспортируемый символ, вызываемый android-activity glue.
// Реализация определяется в каждом приложении.
