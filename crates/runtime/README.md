# egui-android-runtime

**Реактивный runtime для egui-приложений на Android.**

Крейт соединяет Platform и UI: управляет главным циклом, состоянием,
диспатчем сообщений и уведомлениями об изменениях.

[![crates.io](https://img.shields.io/crates/v/egui-android-runtime)](https://crates.io/crates/egui-android-runtime)

## Состав

### `Application` (trait)
- Корень DI: владеет RootComponent, каналами, AppConfig
- `create()` — инициализация
- `frame(ctx, raw_input) -> FullOutput` — один кадр
- `config() / config_mut()` — настройки (log_tag, target_fps)
- `create_notifier(ctx, wake) -> Option<UiNotifier>` — инфраструктура уведомлений

### `Dispatcher<M>`
- Абстракция над `mpsc::Sender`
- `dispatch(msg)` — отправляет сообщение в момент события
- `Clone` — можно передавать дочерним компонентам
- Создаётся каждый кадр, живёт один кадр

### `StateStore<T>`
- Реактивное состояние на `tokio::sync::watch`
- `update(f)` — атомарное изменение + уведомление
- `dispatch(msg, reducer)` — MVI-диспатч
- `state() -> T` — snapshot

### `UiNotifier`
- Инфраструктурный уведомитель
- `check()` — при сигнале вызывает `request_repaint()` + `waker.wake()`
- Не знает про Domain, Components, Reducer

### `AppConfig`
- Настройки приложения: log_tag, target_fps

## Поток данных

```
UI (нажатие кнопки)
  → dispatch(Msg)
    → Receiver накапливает
      ← после render: drain → Component::handle()
        → Data Layer → store.update()
          → notify_tx.send(())
            → UiNotifier::check() → request_repaint()
              → frame() → render(state, &dispatcher)
```

## Когда использовать

Подключайте `egui-android-runtime`, если вы:
- пишете Application (главный цикл, DI)
- работаете с состоянием (StateStore)
- используете Dispatcher для MVI-потока
