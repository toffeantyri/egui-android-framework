# egui-android-core

Базовые MVI-примитивы для egui-приложений на Android.

## Проблема

egui — immediate-mode GUI. В нём нет:
- единого способа организовать код в компоненты с состоянием
- системы сообщений между View и бизнес-логикой
- управления жизненным циклом экранов
- встроенной обработки системной кнопки Back

Этот крейт предоставляет все эти примитивы.

## Состав

| Тип | Описание |
|---|---|
| **`Component`** | Узел дерева навигации. Хранит snapshot состояния, `render()` делегирует View, `handle()` обрабатывает сообщения |
| **`ViewFn<S, M>`** | Чистая функция `(state, ui, dispatch) → ()`. View не имеет побочных эффектов |
| **`Widget<M>`** | Трейт для виджетов и модификаторов. Принимает `&mut UiWrapper` (Compose-like BoxWithConstraints) |
| **`LifecycleObserver`** | Трейт с методами `on_create / on_start / on_resume / on_pause / on_stop / on_destroy` (все с пустой реализацией) |
| **`ComponentContext`** | Контекст компонента: `BackDispatcher`, `back_fallback`, `StateStore`, каналы навигации и data layer |
| **`BackDispatcher`** | Менеджер обработки системной кнопки Back с приоритетами. Аналог `BackDispatcher` из Decompose |
| **`BackCallback`** | Callback для BackDispatcher: `{ priority: u32, handler: Box<dyn FnMut() -> bool> }` |
| **`UiWrapper`** | Обёртка над `egui::Ui` с поддержкой `Constraints` (min/max width/height) |
| **`Constraints`** | Compose-like ограничения размера: `{ min_width, max_width, min_height, max_height }` |

### `ComponentContext::on_back()` — обработка Back

```
ComponentContext::on_back()
  ├── BackDispatcher::handle() — зарегистрированные callback'и (приоритет)
  └── back_fallback — pop из ChildStack или завершение
```

**Внимание:** callback в `BackDispatcher` может пережить зарегистрировавший его компонент. Для экранов с кастомной логикой используйте прямой вызов `handle_back()` из `RootComponent::on_back()`, без BackDispatcher.

## Зависимости

- `egui` — GUI
- `egui-android-runtime` — Dispatcher, StateStore
- `log`

## Пример

```rust
use egui_android_core::{Component, LifecycleObserver, ComponentContext};

struct MyComponent { count: u32, ctx: ComponentContext<NavEvent, DataCmd, AppState> }

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
        self.ctx.send_cmd(msg); // команда в data layer
    }

    fn state(&self) -> &u32 { &self.count }
    fn sync_from_store(&mut self) { /* обновить snapshot из store */ }
}
```
