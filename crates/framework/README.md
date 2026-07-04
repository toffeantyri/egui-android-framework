# egui-android-framework

Umbrella крейт. Re-export всех крейтов фреймворка.

```rust
use egui_android_framework::{
    core::*,
    ui::*,
    runtime::{Application, Dispatcher, StateStore},
    navigation::ChildStack,
    platform::Platform,
    platform_android::run,
};
```

Эквивалентно подключению каждого крейта отдельно.

## Состав

| Модуль | Крейт |
|---|---|
| `egui_android_framework::core` | egui-android-core |
| `egui_android_framework::ui` | egui-android-ui |
| `egui_android_framework::runtime` | egui-android-runtime |
| `egui_android_framework::navigation` | egui-android-navigation |
| `egui_android_framework::platform` | egui-android-platform |
| `egui_android_framework::platform_android` | egui-android-platform-android |
