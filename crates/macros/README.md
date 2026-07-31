# egui-android-macros

**Процедурные макросы для egui-android-framework.**

Генерирует реализацию `ComponentNode` и `PersistentState` для ваших компонентов —
избавляет от шаблонного кода.

[![crates.io](https://img.shields.io/crates/v/egui-android-macros)](https://crates.io/crates/egui-android-macros)

## Проблема

Чтобы использовать `ChildStack`, каждый компонент должен реализовать `ComponentNode`:
`render()`, `handle_dyn()`, `as_any()`, `as_any_mut()` — и всё это вручную.
А если нужно ещё сохранять состояние — ещё и `PersistentState` с ручной сериализацией.

Макросы автоматически генерируют весь этот код.

## Макросы

### `#[derive(ComponentNode)]`

Генерирует реализацию `ComponentNode` для вашей структуры.
Требует указания типа сообщения через `#[component_message(MsgType)]`:

```rust
use egui_android_macros::ComponentNode;

#[derive(ComponentNode)]
#[component_message(MyMsg)]
struct MyScreen;
```

Генерирует: `render()`, `handle_dyn()`, `handle_back()` (со `ctx`), `save_state()`, `restore_state()`, `as_any()`, `as_any_mut()`. Все методы принимают `ComponentContext` (`ctx`), через который экран запрашивает навигацию назад

### `#[derive(Component)]`

Генерирует реализацию `PersistentState` для структуры-компонента.
Сохраняемые поля указываются через `#[persistent_fields(field1, field2)]`:

```rust
use egui_android_macros::{Component, ComponentNode};

#[derive(Component, ComponentNode)]
#[persistent_fields(counter, user_name)]
#[component_message(MyMsg)]
struct MyScreen {
    counter: i32,
    user_name: String,
    temp_buffer: Vec<u8>,  // не сохраняется
}
```

Генерирует тип `__{Name}PersistentState` с `Serialize`/`Deserialize`, и методы
`save()` / `restore()` для полей из `persistent_fields`.

### `#[component]` (attribute)

Оставляет код без изменений. Существует для обратной совместимости.

## Пример полный

```rust
use egui_android_macros::{Component, ComponentNode};
use egui_android_core::{Component, ComponentContext, LifecycleObserver, UiWrapper};
use egui_android_runtime::Dispatcher;

#[derive(Clone, Debug)]
enum MyMsg { Click }

#[derive(Component, ComponentNode)]
#[persistent_fields(counter)]
#[component_message(MyMsg)]
struct CounterScreen {
    counter: i32,
}

impl LifecycleObserver for CounterScreen {}

impl egui_android_core::Component for CounterScreen {
    type State = i32;
    type Message = MyMsg;

    fn render(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<MyMsg>, ctx: &ComponentContext) {
        // ...
    }

    fn handle(&mut self, msg: MyMsg, ctx: &mut ComponentContext) {
        match msg {
            MyMsg::Click => self.counter += 1,
        }
    }

    fn state(&self) -> &i32 { &self.counter }
}
```

## Когда использовать

Всегда, когда вы пишете компонент для `ChildStack`.
Макросы re-экспортируются через [`egui-android-framework`](https://crates.io/crates/egui-android-framework):

```rust
use egui_android::{Component, ComponentNode};
```

## Зависимости

Зависит от: `syn`, `quote`, `proc-macro2`

Dev-зависимости: [`egui-android-core`](https://crates.io/crates/egui-android-core), [`egui-android-runtime`](https://crates.io/crates/egui-android-runtime), [`egui-android-framework`](https://crates.io/crates/egui-android-framework)
