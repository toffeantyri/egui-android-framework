# egui-android-platform-android

**Реализация Platform для Android: EGL, input, главный цикл, lifecycle.**

Связывает NativeActivity, EGL и egui_glow в единый цикл.
Обрабатывает Android lifecycle, touch-ввод (с инерцией),
системные панели (status bar, navigation bar) и кнопку Back.

[![crates.io](https://img.shields.io/crates/v/egui-android-platform-android)](https://crates.io/crates/egui-android-platform-android)

## Возможности

- **Главный цикл** — EGL + poll_events() + egui_glow + lifecycle
- **Touch-ввод** — MotionEvent → egui::Event. Скролл с инерцией (fling)
- **Батчинг событий** — Down на первом кадре, Move/Up на следующих. Предотвращает скачки scroll_offset
- **Кнопка Back** — перехват AKEYCODE_BACK, делегирование в Application::on_back_pressed()
- **Системные панели** — установка цвета status bar / navigation bar через JNI, корректировка insets для MIUI
- **Lifecycle** — InitWindow, Resume, Pause, Stop, Destroy

## Использование

```rust
use egui_android_platform_android::run;

#[no_mangle]
fn android_main(app: android_activity::AndroidApp) {
    run::<MyApplication>(app);
}
```

## Зависимости

- EGL через glow
- OpenGL через egui_glow
- JNI для системных панелей
- ndk для NativeWindow
