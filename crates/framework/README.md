# egui-android-framework

Umbrella-крейт. Re-export всех крейтов фреймворка в одном месте.

Не содержит собственной логики — только `pub use` всех публичных API дочерних крейтов.

## Состав

| Модуль | Крейт | Назначение |
|---|---|---|
| `egui_android_framework::core` | egui-android-core | MVI примитивы: Component, ViewFn, LifecycleObserver, BackDispatcher, UiWrapper |
| `egui_android_framework::ui` | egui-android-ui | Виджеты, модификаторы, контейнеры, анимации, темы, remember |
| `egui_android_framework::runtime` | egui-android-runtime | Application, Dispatcher, StateStore, UiNotifier |
| `egui_android_framework::navigation` | egui-android-navigation | ChildStack с управлением жизненным циклом |
| `egui_android_framework::platform` | egui-android-platform | Абстрактный Platform trait |
| `egui_android_framework::platform_android` | egui-android-platform-android | Android: EGL, input, главный цикл, Back |

## Использование

```rust
use egui_android_framework::{
    core::{Component, LifecycleObserver, BackDispatcher, UiWrapper},
    ui::prelude::*,
    runtime::{Application, Dispatcher, StateStore},
    navigation::ChildStack,
    platform_android::run,
};
```

Эквивалентно подключению каждого крейта отдельно, но удобнее — достаточно одной зависимости в Cargo.toml:

```toml
[dependencies]
egui-android-framework = "0.3"
```
