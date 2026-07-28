# egui-android

**cdylib-обёртка для Android-приложения на egui.**

Экспортирует символ `android_main`, который вызывается `android-activity` glue
при запуске приложения. Конкретная реализация `Application` определяется в каждом
приложении отдельно.

[![crates.io](https://img.shields.io/crates/v/egui-android)](https://crates.io/crates/egui-android)

> **Важно:** Этот крейт помечен `publish = false` — он не предназначен для публикации.
> Используйте его как шаблон для своего приложения, скопировав структуру из
> [`examples/counter`](https://github.com/toffeantyri/egui-android-framework/tree/master/examples/counter)
> или [`examples/showcase`](https://github.com/toffeantyri/egui-android-framework/tree/master/examples/showcase).

## Структура

Каждое конкретное приложение (showcase, counter) должно определить свою реализацию
`Application` и вызывать её из `android_main`. Этот крейт — только обёртка для
экспорта символа `android_main`.

## Использование (как шаблон)

Создайте свой крейт с `crate-type = ["cdylib"]` и точкой входа:

```rust
use egui_android::platform_android::run;
use my_app::MyApplication;

#[no_mangle]
fn android_main(app: android_activity::AndroidApp) {
    run::<MyApplication>(app);
}
```

## Зависимости

- [`egui-android-platform-android`](https://crates.io/crates/egui-android-platform-android) — главный цикл, EGL, input
- [`egui-android-runtime`](https://crates.io/crates/egui-android-runtime) — Application trait
- `android-activity` — GameActivity glue
- `android_logger` — логирование в logcat
