# egui-android-runtime

**Реактивный runtime для egui-приложений на Android.**

Соединяет Platform и UI: управляет главным циклом, состоянием,
диспатчем сообщений и уведомлениями об изменениях.

[![crates.io](https://img.shields.io/crates/v/egui-android-runtime)](https://crates.io/crates/egui-android-runtime)

## Проблема

В egui-приложении на Android нужно:
- Организовать DI и главный цикл
- Управлять состоянием и уведомлять UI об изменениях
- Отправлять сообщения от View в Component и Data Layer

Этот крейт предоставляет всю эту инфраструктуру.

## Состав

### `Application` (trait)
- Корень DI: владеет RootComponent, каналами, Data Layer
- `create()` — инициализация приложения
- `frame(ctx, raw_input) -> FullOutput` — один кадр
- `config() / config_mut()` — настройки (log_tag, target_fps)
- `on_back_pressed()` — обработка системной кнопки Back
- `on_save_state() / on_restore_state()` — save/restore навигации
- `create_runtime_context()` — связь с платформой (Waker)
- `request_destroy()` — запрос завершения приложения
- Методы жизненного цикла: `on_create`, `on_start`, `on_resume`, `on_pause`, `on_stop`, `on_destroy`

### `RuntimeConfig`
- Настройки приложения: `log_tag`, `target_fps`
- `Default` — 60 FPS, тег "egui_app"

### `DataLayerHandle<DataCmd>`
- Handle для отправки команд в фоновый Data Layer
- `send(cmd)` — отправить команду
- `sender()` — получить `mpsc::Sender<DataCmd>` для передачи в `ComponentContext`

### `Dispatcher<M>`
- Абстракция над `mpsc::Sender`
- `dispatch(msg)` — отправляет сообщение в момент события
- `Clone` — можно передавать дочерним компонентам
- Создаётся каждый кадр, живёт один кадр

### `DynDispatcher`
- Type-erased версия `Dispatcher` для `Box<dyn ComponentNode>`
- `wrap::<M>()` — получить типизированный `Dispatcher<M>`
- Используется в `ChildStack` для передачи сообщений активному компоненту

### `MessageEnvelope<M>`
- Типобезопасная обёртка для сообщений
- Гарантирует `M: Clone + Debug + Send + 'static`
- Ошибки downcast логируются с указанием ожидаемого типа

### `StateStore<T>`
- Реактивное состояние на `tokio::sync::watch`
- `update(f)` — атомарное изменение + уведомление
- `state() -> T` — snapshot

### `RuntimeContext`
- Контекст выполнения для платформы
- `check()` — единственный публичный метод, вызывает `request_repaint()` + `waker.wake()`
- Инкапсулирует `UiNotifier` от платформы

### `SavedState` / `SavedStack<C>`
- Типы для save/restore навигации (Decompose-style)
- `SavedStack` — сериализуемый стек с конфигурациями компонентов
- Используется в `ChildStack::save()` / `restore()`

## Поток данных

```
UI (нажатие кнопки)
  → dispatch(Msg)
    → ui_msg_rx накапливает
      ← после render: drain → Component::handle()
        → Data Layer → store.update()
          → data_statechanged_tx.send(())
            → RuntimeContext::check() → request_repaint()
              → frame() → render(state, &dispatcher)
```

## Пример

```rust
use egui_android_runtime::{
    Application, RuntimeConfig, RuntimeContext, StateStore, DataLayerHandle,
};
use egui_android_platform::Waker;

struct MyApp {
    config: RuntimeConfig,
    store: StateStore<AppState>,
}

impl Application for MyApp {
    type RootComponent = MyRoot;

    fn create() -> Self {
        let config = RuntimeConfig { log_tag: "myapp".into(), target_fps: 60 };
        let store = StateStore::new(AppState::default());
        Self { config, store }
    }

    fn root(&mut self) -> &mut MyRoot { &mut self.root }
    fn root_ref(&self) -> &MyRoot { &self.root }
    fn config(&self) -> &RuntimeConfig { &self.config }
    fn config_mut(&mut self) -> &mut RuntimeConfig { &mut self.config }

    fn on_back_pressed(&mut self) { self.root.on_back(); }
}
```

## Когда использовать

Подключайте `egui-android-runtime`, если вы:
- пишете `Application` (главный цикл, DI)
- работаете с состоянием (`StateStore`)
- используете `Dispatcher` для MVI-потока

Для обычного использования все типы доступны через [`egui-android-framework`](https://crates.io/crates/egui-android-framework):

```rust
use egui_android::runtime::{Application, RuntimeConfig, StateStore};
```

## Зависимости

Зависит от: [`egui-android-platform`](https://crates.io/crates/egui-android-platform), `egui`, `tokio`, `serde`, `bincode`

От него зависят: [`egui-android-core`](https://crates.io/crates/egui-android-core), [`egui-android-platform-android`](https://crates.io/crates/egui-android-platform-android)
