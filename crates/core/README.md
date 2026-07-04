# egui-android-core

MVI примитивы: Component, ViewFn, LifecycleObserver, Widget.

## Типы

- **`Component`** — узел дерева навигации с состоянием и обработкой сообщений
- **`ViewFn<S, M>`** — чистая функция `(state, ui, dispatch) → ()`
- **`LifecycleObserver`** — трейт с методами `on_create/on_start/on_resume/...`
- **`Widget<M>`** — трейт для виджетов и модификаторов
- **`ComponentContext`** — контекст компонента (навигация, DI)

## Пример

```rust
use egui_android_core::{Component, LifecycleObserver};

struct MyComponent { count: u32 }

impl LifecycleObserver for MyComponent {}

impl Component for MyComponent {
    type State = u32;
    type Message = Msg;

    fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<Msg>) {
        ui.label(format!("{}", self.count));
        if ui.button("+1").clicked() {
            dispatch.dispatch(Msg::Increment);
        }
    }

    fn handle(&mut self, msg: Msg) {
        // отправить команду в data layer
    }

    fn state(&self) -> &u32 { &self.count }
    fn sync_from_store(&mut self) { /* обновить snapshot */ }
}
```
