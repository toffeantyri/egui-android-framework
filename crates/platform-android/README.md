# egui-android-platform-android

Android-реализация Platform: EGL, NDK input, главный цикл.

## Возможности

- **EGL** — прямой доступ к libEGL.so (OpenGL ES 2.0), без glutin
- **NDK Input** — MotionEvent → egui::Event (PointerButton + Touch)
- **Главный цикл** — событийный: `poll_events()` + `UiNotifier::check()` + `frame()`
- **Back кнопка** — встроенная обработка
- **MIUI workaround** — корректировка content_rect для Xiaomi/MIUI

## Использование

```rust
use egui_android_platform_android::run;

#[no_mangle]
pub fn android_main(app: AndroidApp) {
    run::<MyApp>(app);
}
```
