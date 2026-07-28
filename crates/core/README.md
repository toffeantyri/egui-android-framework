# egui-android-core

**MVI-примитивы для egui-приложений на Android.**

Содержит базовые трейты для построения UI-компонентов с однонаправленным потоком данных:
`Component`, `ComponentNode`, `LifecycleObserver`, `PersistentState`, `BackDispatcher`,
а также инфраструктурные элементы: `UiWrapper`, `Constraints`, `ComponentContext`.

[![crates.io](https://img.shields.io/crates/v/egui-android-core)](https://crates.io/crates/egui-android-core)

## Проблема

egui — immediate-mode GUI, где состояние хранится в замыканиях. Для большого приложения это
приводит к спагетти-коду. Нужна архитектура: компоненты с чётким жизненным циклом,
изолированные сообщения, сохранение состояния при повороте экрана.

Этот крейт даёт строительные блоки для такой архитектуры.

## Состав

### `Component` — узел дерева навигации
- Владеет состоянием + обрабатывает сообщения
- `render(ui, dispatch)` — рендеринг View
- `handle(msg)` — обработка сообщения (команда в data layer или из View)
- `state() -> &State` — snapshot состояния
- Ассоциированные типы: `State`, `Message: Clone + Debug + Send + 'static`

### `ComponentNode` — object-safe трейт для хранения в `ChildStack`
- Позволяет складывать в `Vec<Box<dyn ComponentNode>>` компоненты с разными типами сообщений
- `render(&self, ui, &DynDispatcher)` — type-erased render
- `handle_dyn(msg)` — type-erased handle с downcast
- `handle_back() -> bool` — Decompose-style обработка Back
- `take_back_request() -> bool` — запрос навигации назад после `handle()`
- `save_state() / restore_state()` — save/restore для пересоздания Activity
- Реализуется через `#[derive(ComponentNode)]` (из `egui-android-macros`)

### `LifecycleObserver`
- `on_create / on_start / on_resume / on_pause / on_stop / on_destroy`
- Все методы имеют пустую реализацию по умолчанию (opt-in)

### `PersistentState`
- Типобезопасное save/restore состояния компонента
- Сериализация через `bincode` + `serde`
- Хелперы `save_to_boxed()` / `restore_from_boxed()` для `ComponentNode`
- Реализуется через `#[derive(Component)]` с `#[persistent_fields(...)]`

### `ComponentContext`
- Контекст компонента (не generic)
- `back_dispatcher: BackDispatcher` — регистрация кастомных обработчиков Back
- `finish_requested: bool` — флаг завершения приложения

### `BackDispatcher`
- Центральный менеджер кнопки Back
- Регистрация callback'ов: `register(callback)` / `unregister_all()`
- `dispatch()` — запуск цепочки обработчиков

### `UiWrapper` — обёртка над `egui::Ui`
- Хранит `Constraints` (доступны через `Context::data()`)
- `Deref<Target = egui::Ui>` — полная совместимость
- `allocate_space(size)` — alloc с учётом constraints

### `Constraints` — Compose-like ограничения размера
- `min_width`, `max_width`, `min_height`, `max_height`
- `exact(w, h)` / `ranged(min, max)` / `unconstrained()`

## Пример

```rust
use egui_android_core::{Component, ComponentNode, LifecycleObserver, UiWrapper, PersistentState};
use egui_android_runtime::Dispatcher;

#[derive(Component, ComponentNode)]
#[persistent_fields(counter)]
#[component_message(Msg)]
struct CounterScreen { counter: i32 }

#[derive(Clone, Debug)]
enum Msg { Increment, Reset }

impl LifecycleObserver for CounterScreen {}

impl Component for CounterScreen {
    type State = i32;
    type Message = Msg;

    fn render(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<Msg>) {
        // рендеринг
    }

    fn handle(&mut self, msg: Msg) {
        match msg {
            Msg::Increment => self.counter += 1,
            Msg::Reset => self.counter = 0,
        }
    }

    fn state(&self) -> &i32 { &self.counter }
}
```

## Когда использовать

Подключайте `egui-android-core`, если вы:
- пишете свои компоненты (`Component`, `ComponentNode`)
- работаете с навигацией и жизненным циклом
- реализуете кастомную обработку Back

Для обычного использования все типы доступны через [`egui-android-framework`](https://crates.io/crates/egui-android-framework):

```rust
use egui_android::core::{Component, ComponentNode, UiWrapper};
use egui_android::Component;  // derive-макрос
```

## Зависимости

Зависит от: [`egui-android-runtime`](https://crates.io/crates/egui-android-runtime), `egui`, `serde`, `bincode`

От него зависят: [`egui-android-ui`](https://crates.io/crates/egui-android-ui), [`egui-android-navigation`](https://crates.io/crates/egui-android-navigation)
