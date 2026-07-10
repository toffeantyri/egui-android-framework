# egui-android-platform-android

Android-реализация платформы: EGL, NDK input, главный цикл, обработка Back.

## Проблема

Запуск egui на Android требует:
- интеграции с NativeActivity (android-activity)
- создания EGL context + surface (libEGL.so, OpenGL ES 2.0)
- трансляции Android input (MotionEvent, KeyEvent) в egui::Event
- событийного главного цикла с poll_events() вместо бесконечного loop
- корректной обработки Android lifecycle (InitWindow, Resume, Pause, Destroy)
- обработки системной кнопки Back

Этот крейт реализует всё перечисленное.

## Возможности

- **EGL** — прямой доступ к libEGL.so и libGLESv2.so, без glutin
- **NDK Input** — MotionEvent (touch) → egui::Event (PointerButton + Touch), клавиатура, Back
- **Главный цикл** — событийный: `poll_events(16ms)` + `UiNotifier::check()` + `frame()`
- **Back кнопка** — перехват AKEYCODE_BACK, делегирование в `Application::on_back_pressed()`
- **MIUI workaround** — корректировка content_rect и insets для Xiaomi/MIUI
- **Insets** — получение system window insets через JNI
- **Lifecycle** — InitWindow (пересоздание surface), Resume/Pause/Stop/Destroy

## Использование

```rust
use egui_android_platform_android::run;

#[no_mangle]
pub fn android_main(app: AndroidApp) {
    run::<MyApp>(app);
}
```

## Обработка Back

Перехват AKEYCODE_BACK происходит в `input.rs`:
- `action == KeyAction::Down && code == Keycode::Back` → `state.back_pressed = true`
- Возвращается `InputStatus::Handled` — Android не вызывает системное поведение

В главном цикле (`run.rs`):
- После слива input-событий проверяется `back_pressed`
- Вызывается `app_instance.on_back_pressed()`
- Platform не знает про навигацию — только делегирует Application

## Зависимости

- `egui` — GUI
- `egui_glow` — OpenGL рендеринг
- `glow` — OpenGL context
- `android-activity` — NativeActivity bindings
- `ndk` — NDK bindings
- `libc` — dlopen/dlsym для EGL
- `jni` — получение system insets
- `android_logger` — логирование в logcat
- `egui-android-platform` — Platform trait
- `egui-android-runtime` — Application trait
