# egui-android-navigation

**Навигация с управлением жизненным циклом для egui-приложений на Android.**

ChildStack — стек компонентов с управлением жизненным циклом,
аналог ChildStack из Decompose.

[![crates.io](https://img.shields.io/crates/v/egui-android-navigation)](https://crates.io/crates/egui-android-navigation)

## Возможности

- **push / pop / replace / bring_to_front / clear**
- **Жизненный цикл** — при push/pop вызываются `on_create / on_destroy`
- **pop()** возвращает `Option<(C, Comp)>` — `None` если стек пуст

## Пример

```rust
use egui_android_navigation::ChildStack;

let mut stack = ChildStack::new();
stack.push("home", HomeComponent::new());
stack.push("settings", SettingsComponent::new());

if let Some((key, comp)) = stack.pop() {
    // вернулись на Home
}
```

## Когда использовать

Подключайте `egui-android-navigation`, если вам нужна
навигация с историей переходов и жизненным циклом экранов.
