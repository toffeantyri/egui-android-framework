# egui-android-framework

MVI-фреймворк для запуска **egui**-приложений на Android с реактивной архитектурой в стиле Jetpack Compose.

**Однонаправленный поток данных через Dispatcher.** Никакого polling. Состояние публикует себя само.

## Состав

Фреймворк состоит из 7 крейтов:

| Крейт | crates.io | Описание |
|---|---|---|
| [egui-android-core] | — | MVI примитивы: Component, ViewFn, LifecycleObserver |
| [egui-android-ui] | — | Виджеты, модификаторы, remember, анимации, темы |
| [egui-android-runtime] | — | Application, Dispatcher, StateStore, UiNotifier |
| [egui-android-navigation] | — | ChildStack, управление жизненным циклом экранов |
| [egui-android-platform] | — | Абстрактный Platform trait |
| [egui-android-platform-android] | — | Android реализация: EGL, NDK input, главный цикл |
| [egui-android-framework] | — | Umbrella крейт, re-export всего |

## Быстрый старт

Смотри [examples/counter](examples/counter) — минимальное приложение на Compose-like API.

```bash
cd examples/counter
x run --device adb:XXXXXXXX
```

## Архитектура

```
UI (нажатие кнопки)
  → dispatch.dispatch(Msg::Increment)
    → Receiver (mpsc) накапливает сообщения
      ← после render: drain receiver
        → Component::handle(msg) отправляет команду в data layer
          → Data Layer → store.update(|s| s.count += 1)
            → notify_tx.send(())
              → UiNotifier::check()
                → ctx.request_repaint() → waker.wake()
                  → frame() → render(state, &dispatcher)
```

## Зависимости

| Крейт | Версия |
|---|---|
| egui | 0.35 |
| egui_glow | 0.35 |
| glow | 0.17 |
| android-activity | 0.6 |
| ndk | 0.9 |
| tokio | 1 (sync) |

## Лицензия

MIT или Apache 2.0
