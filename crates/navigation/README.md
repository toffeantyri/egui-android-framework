# egui-android-navigation

Навигация: стек дочерних компонентов с управлением жизненным циклом.

## Проблема

В egui нет встроенной навигации между экранами. Ручное управление стеком экранов приводит к:
- утечкам ресурсов (не вызываются lifecycle-методы при уничтожении)
- сложному коду при push/pop/replace
- отсутствию typed-конфигурации экранов

`ChildStack` решает все эти проблемы — это аналог `ChildStack` из Decompose.

## Возможности

- **`push(config, component)`** — добавить компонент на вершину стека (вызывает lifecycle: create → start → resume)
- **`pop() -> Option<(C, Comp)>`** — убрать верхний элемент (lifecycle: pause → stop → destroy)
- **`replace(config, component)`** — заменить верхний (pop + push)
- **`bring_to_front(config, component)`** — переместить стек к указанному состоянию
- **`clear()`** — очистить стек (уничтожить все компоненты)
- **`active() -> Option<&Comp>`** — активный (верхний) компонент
- **`active_mut() -> Option<&mut Comp>`** — мутабельная ссылка на активный
- **`active_config() -> Option<&C>`** — конфигурация активного компонента
- **`is_empty() / len()`** — состояние стека

Все `push/pop/replace/clear/bring_to_front` автоматически вызывают lifecycle-методы
`LifecycleObserver` (on_create, on_start, on_resume, on_pause, on_stop, on_destroy).

## Зависимости

- `egui` — GUI
- `egui-android-core` — `Component`, `LifecycleObserver`, `UiWrapper`
- `egui-android-ui` — виджеты и контейнеры для render
- `log`

## Пример

```rust
use egui_android_navigation::ChildStack;

let mut stack = ChildStack::<Screen, MyComponent>::new();

stack.push(Screen::Home, MyComponent::new("home"));
stack.push(Screen::Details, MyComponent::new("details"));

if let Some(active) = stack.active() {
    // рендерим активный экран
}

stack.pop(); // возвращает (Screen::Details, MyComponent)
```
