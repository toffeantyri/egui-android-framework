# egui-android-navigation

Навигация: ChildStack, управление жизненным циклом дочерних компонентов.

## Возможности

- `push`, `pop`, `replace`, `bring_to_front`, `clear`
- Автоматическое управление жизненным циклом (LifecycleObserver)
- typed компоненты с состоянием и сообщениями

## Пример

```rust
use egui_android_navigation::ChildStack;

let mut stack = ChildStack::<Screen, MyComponent>::new(MyComponent::new("home"));

stack.push(Screen::Details, MyComponent::new("details"));
stack.pop();
```
