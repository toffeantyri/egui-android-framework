# egui-android-framework

**MVI-фреймворк для egui на Android.** Позволяет писать нативные Android-приложения на Rust с GUI через egui, используя реактивную архитектуру в стиле Jetpack Compose.

[![crates.io](https://img.shields.io/crates/v/egui-android-framework)](https://crates.io/crates/egui-android-framework)
[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](#лицензия)

## Проблема

egui — отличный immediate-mode GUI, но для создания полноценного Android-приложения на нём не хватает инфраструктуры:

- **Главный цикл** — нужно интегрировать `egui_glow` с NativeActivity, EGL, Android lifecycle
- **Ввод** — Android MotionEvent → события egui (touch, клавиатура, back)
- **Архитектура** — как организовать код: экраны, навигация, состояние, бизнес-логика
- **Реактивность** — UI должен обновляться при изменении данных, а не по таймеру

Этот фреймворк решает все эти проблемы «из коробки».

## Решение

```
┌─────────────────────────────────────────────────────┐
│                  egui-android-framework              │
│                                                     │
│  ┌──────────┐  ┌──────┐  ┌─────────┐  ┌──────────┐ │
│  │ platform │  │ core │  │   ui    │  │navigation│ │
│  │  - EGL   │  │ Comp │  │ Widgets │  │ ChildS.  │ │
│  │  - Input │  │ View │  │ Anim.   │  │ Lifecyc. │ │
│  │  - Loop  │  │ Life │  │ Theme   │  │          │ │
│  └────┬─────┘  └──┬───┘  └────┬────┘  └────┬─────┘ │
│       └───────────┼───────────┼────────────┘        │
│                   ▼           ▼                      │
│          ┌──────────────────────────┐                │
│          │        runtime           │                │
│          │  Application | Dispatcher│                │
│          │  StateStore | UiNotifier │                │
│          └──────────────────────────┘                │
└─────────────────────────────────────────────────────┘
```

**Ключевые возможности:**

- ✅ **Единый главный цикл** — EGL + `poll_events()` + `egui_glow` + Android lifecycle
- ✅ **Touch-ввод** — MotionEvent → egui::Event, поддержка скролла с инерцией (fling)
- ✅ **MVI-архитектура** — Component + ViewFn + Dispatcher + StateStore
- ✅ **Реактивное состояние** — `StateStore<T>` на `tokio::sync::watch`, автоматический repaint
- ✅ **Compose-like UI** — Column, Row, Stack, LazyColumn, модификаторы, анимации
- ✅ **Навигация** — ChildStack с управлением жизненным циклом экранов
- ✅ **Темы** — Material Design 3 (light/dark)
- ✅ **Кнопка Back** — встроенная обработка
- ✅ **MIUI support** — корректировка отступов для Xiaomi
- ✅ **Минимальный APK** — opt-level = z, LTO, strip

## Быстрый старт

```bash
# Установите xbuild
cargo install xbuild

# Клонируйте пример и запустите
git clone https://github.com/toffeantyri/egui-android-framework
cd egui-android-framework/examples/counter
x run --device adb:XXXXXXXX
```

### Минимальный код приложения

```rust
// lib.rs
use egui_android_framework::{
    core::*,
    ui::prelude::*,
    runtime::{Application, Dispatcher, StateStore, AppConfig},
    platform_android::run,
};

struct CounterComponent { count: u32 }

impl Component for CounterComponent {
    type State = u32;
    type Message = Msg;

    fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<Msg>) {
        Column::new(ui, dispatch, |ui, dispatch| {
            Text::new(format!("{}", self.count))
                .padding(16.0)
                .render(ui, dispatch);
            Button::new("+1")
                .on_click(Msg::Increment)
                .render(ui, dispatch);
        });
    }

    fn handle(&mut self, msg: Msg) { /* команда в data layer */ }
    fn state(&self) -> &Self::State { &self.count }
    fn sync_from_store(&mut self) { /* обновить snapshot из Store */ }
}
```

## Состав крейтов

| Крейт | crates.io | Назначение |
|---|---|---|
| egui-android-core | [![crates.io](https://img.shields.io/crates/v/egui-android-core)](https://crates.io/crates/egui-android-core) | MVI примитивы: Component, ViewFn, LifecycleObserver |
| egui-android-ui | [![crates.io](https://img.shields.io/crates/v/egui-android-ui)](https://crates.io/crates/egui-android-ui) | Виджеты, модификаторы, remember, анимации, темы |
| egui-android-runtime | [![crates.io](https://img.shields.io/crates/v/egui-android-runtime)](https://crates.io/crates/egui-android-runtime) | Application, Dispatcher, StateStore, UiNotifier |
| egui-android-navigation | [![crates.io](https://img.shields.io/crates/v/egui-android-navigation)](https://crates.io/crates/egui-android-navigation) | ChildStack, управление жизненным циклом |
| egui-android-platform | [![crates.io](https://img.shields.io/crates/v/egui-android-platform)](https://crates.io/crates/egui-android-platform) | Абстрактный Platform trait |
| egui-android-platform-android | [![crates.io](https://img.shields.io/crates/v/egui-android-platform-android)](https://crates.io/crates/egui-android-platform-android) | Android: EGL, input, главный цикл |
| egui-android-framework | [![crates.io](https://img.shields.io/crates/v/egui-android-framework)](https://crates.io/crates/egui-android-framework) | Umbrella, re-export всего |

## Технологии

| Зависимость | Версия | Назначение |
|---|---|---|
| egui | 0.35 | GUI |
| egui_glow | 0.35 | OpenGL рендеринг |
| glow | 0.17 | OpenGL context |
| android-activity | 0.6 | NativeActivity |
| ndk | 0.9 | NDK bindings |
| tokio (sync) | 1 | watch-каналы для StateStore |

## Архитектура приложения

```
                    ┌──────────────────────┐
                    │      Application      │
                    │   (корень DI, каналы) │
                    └──────┬───────────────┘
                           │
        ┌──────────────────┼──────────────────┐
        │                  │                  │
        ▼                  ▼                  ▼
┌──────────────┐   ┌──────────────┐   ┌──────────────┐
│  Navigation  │   │     UI       │   │    Core      │
│  ChildStack  │   │  Виджеты     │   │  Component   │
│  Lifecycle   │   │  Модификаторы│   │  Widget<M>   │
└──────┬───────┘   └──────┬───────┘   └──────┬───────┘
       │                  │                  │
       └──────────────────┼──────────────────┘
                          │
                          ▼
                   ┌──────────────┐
                   │   Runtime     │
                   │ Dispatcher    │
                   │ StateStore    │
                   │ UiNotifier    │
                   └───────┬───────┘
                           │
              ┌────────────┴────────────┐
              │                         │
              ▼                         ▼
      ┌──────────────┐       ┌──────────────────┐
      │   Platform   │       │ Platform-Android  │
      │  (trait)     │◀──────│ EGL | Input      │
      └──────────────┘       │ run<A>()          │
                             └──────────────────┘
```

## Лицензия

MIT или Apache 2.0
