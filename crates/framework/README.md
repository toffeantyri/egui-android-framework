# egui-android-framework

**Umbrella-крейт. Re-export всех крейтов egui-android.**

Содержит только `pub use` всех крейтов workspace.
Подключайте этот крейт для простоты — он даёт доступ ко всему фреймворку.

[![crates.io](https://img.shields.io/crates/v/egui-android-framework)](https://crates.io/crates/egui-android-framework)

## Состав

- `egui-android-core` — Component, Widget, UiWrapper, Constraints, BackDispatcher
- `egui-android-ui` — виджеты, контейнеры, модификаторы, анимации, темы
- `egui-android-runtime` — Application, Dispatcher, StateStore
- `egui-android-navigation` — ChildStack
- `egui-android-platform` — абстрактный Platform trait
- `egui-android-platform-android` — Android реализация (EGL, input)

## Импорт

```rust
// Через umbrella
use egui_android_framework::runtime::{Application, Dispatcher};
use egui_android_framework::core::{Component, UiWrapper};
use egui_android_framework::ui::{
    containers::{Column, Stack, Align},
    modifier::{Modifier, ModifierDsl},
    widgets::{Button, Text, Widget},
    theme::Theme,
};

// Напрямую (если подключен только конкретный крейт)
use egui_android_runtime::Application;
```
