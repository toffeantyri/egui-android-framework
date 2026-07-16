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
- **UI-компоненты** — нет готовых Column, Row, Stack, LazyColumn, модификаторов, тем
- **Навигация** — нет ChildStack с жизненным циклом экранов

Этот фреймворк решает все эти проблемы «из коробки».

## Решение

```
┌─────────────────────────────────────────────────────┐
│                  egui-android-framework              │
│                                                     │
│  ┌──────────┐  ┌──────────┐  ┌────────┐  ┌────────┐│
│  │ platform │  │   core   │  │   ui   │  │ nav-   ││
│  │  - EGL   │  │ Component│  │ Widgets│  │ igation││
│  │  - Input │  │ ViewFn   │  │ Contain│  │ ChildS.││
│  │  - Loop  │  │ Lifecycle│  │ Modif. │  │ Lifecy.││
│  │          │  │ BackDisp.│  │ Anim.  │  │        ││
│  │          │  │          │  │ Theme  │  │        ││
│  └────┬─────┘  └────┬─────┘  └───┬────┘  └────┬───┘│
│       └─────────────┼────────────┼─────────────┘    │
│                     ▼            ▼                   │
│            ┌──────────────────────────┐              │
│            │        runtime           │              │
│            │  Application | Dispatcher│              │
│            │  StateStore | UiNotifier │              │
│            └──────────────────────────┘              │
└─────────────────────────────────────────────────────┘
```

**Ключевые возможности:**

- ✅ **Единый главный цикл** — EGL + `poll_events()` + `egui_glow` + Android lifecycle
- ✅ **Touch-ввод** — MotionEvent → egui::Event, поддержка скролла с инерцией (fling), батчинг событий для исключения скачков
- ✅ **MVI-архитектура** — Component + ViewFn + Dispatcher + StateStore. Однонаправленный поток данных, реактивное состояние через `tokio::sync::watch`
- ✅ **Compose-like UI** — Column, Row, Stack (с двухфазным measure→layout), LazyColumn. Модификаторы: padding, background, border, clip, shadow, alpha, width, height, width_in, height_in, fill_max_width, clickable, wrap_content, size. Анимации: Fade, Slide, AnimatedVisibility. Тема: Material Design 3 (light/dark)
- ✅ **Навигация** — ChildStack с управлением жизненным циклом экранов (push/pop/replace, on_create/on_destroy)
- ✅ **Кнопка Back** — иерархическая обработка: кастомная логика экрана → BackDispatcher (диалоги) → ChildStack pop → завершение приложения
- ✅ **Темы** — Material Design 3 light/dark с полной палитрой, типографикой, скруглениями. Автоматическое определение системной темы
- ✅ **Системные панели** — автоматическая обработка insets (status bar, navigation bar), смена цвета панелей под тему
- ✅ **Локальное UI-состояние** — `remember<T>()` (аналог Compose `remember`). Хранится между кадрами, не требует мутабельного доступа
- ✅ **MIUI support** — корректировка отступов для Xiaomi устройств
- ✅ **Минимальный APK** — opt-level = z, LTO, strip
- ✅ **Визуальный отклик** — Button меняет цвет при нажатии (pressed ≠ normal). `theme_colors()` автоматически подбирает pressed под любую тему

## Быстрый старт

### 1. Сборка Rust .so

```bash
# Установите cargo-ndk
cargo install cargo-ndk

# Добавьте Android target
rustup target add aarch64-linux-android

# Настройте NDK
# Убедитесь, что ANDROID_NDK_HOME указывает на ваш NDK,
# или укажите ndk.dir в android/local.properties

# Соберите .so для arm64-v8a
ANDROID_NDK_HOME=/usr/lib/android-sdk/ndk/27.3.13750724 \
  cargo ndk -t arm64-v8a -o android/app/src/main/jniLibs build -p egui-gl-app
```

### 2. Сборка APK

```bash
cd android
./gradlew assembleDebug
adb install -r app/build/outputs/apk/debug/app-debug.apk
```

Или одной командой:

```
./scripts/build_android.sh --install
```

### Зависимости для сборки

| Инструмент | Установка |
|---|---|
| Rust Android target | `rustup target add aarch64-linux-android` |
| cargo-ndk | `cargo install cargo-ndk` |
| Android SDK | `sudo apt install android-sdk` или Android Studio |
| Android NDK | `sudo apt install google-android-ndk-r27d-installer` |
| Gradle | `sudo apt install gradle` |
| Kotlin | Встроен в Gradle plugin |
| build-tools | `sdkmanager "build-tools;33.0.0"` |
| platform 34 | `sdkmanager "platforms;android-34"` |

### Минимальный код приложения

```rust
use egui_android_framework::{
    core::*,
    ui::prelude::*,
    runtime::{Application, Dispatcher, StateStore, AppConfig},
    platform_android::run,
};

struct CounterApp;

impl Application for CounterApp {
    type State = u32;
    type Message = Msg;

    fn render(&self, state: &u32, ui: &mut UiWrapper, dispatch: &Dispatcher<Msg>) {
        Column::new().show(ui, dispatch, |ui, dispatch| {
            Text::new(format!("Счёт: {}", state))
                .modifier(Modifier::new().padding(16.0))
                .render(ui, dispatch);
            Button::new("+1")
                .on_click(Msg::Increment)
                .render(ui, dispatch);
            Button::new("Сброс")
                .theme_colors(egui::Color32::RED)
                .text_color(egui::Color32::WHITE)
                .on_click(Msg::Reset)
                .render(ui, dispatch);
        });
    }
}
```├── settings.gradle
├── gradle.properties
└── local.properties               # ← sdk.dir, ndk.dir
```

### Минимальный код приложения

### Пример UI с Compose-like синтаксисом

```rust
Column::new().show(ui, dispatch, |ui, dispatch| {
    // Текст с отступом
    Text::new("Заголовок")
        .modifier(Modifier::new().padding(8.0))
        .render(ui, dispatch);

    // Кнопка на всю ширину с фоном
    Button::new("Действие")
        .theme_colors(c.primary)
        .text_color(c.on_primary)
        .on_click(Msg::Action)
        .modifier(Modifier::new().fill_max_width().padding(8.0))
        .render(ui, dispatch);

    // Локальное состояние (remember)
    let count = remember(ui, "counter", || 0i32);

    // Stack с наложением (двухфазный measure→layout)
    Stack::new()
        .add(Text::new("Фон").modifier(Modifier::new().background(c.surface)))
        .add_with_align(Text::new("По центру"), Align::Center)
        .show(ui, dispatch);
});
```

## Состав крейтов

| Крейт | crates.io | Назначение |
|---|---|---|
| egui-android-core | [![crates.io](https://img.shields.io/crates/v/egui-android-core)](https://crates.io/crates/egui-android-core) | MVI примитивы: Component, ViewFn, Widget, LifecycleObserver, BackDispatcher, UiWrapper, Constraints |
| egui-android-ui | [![crates.io](https://img.shields.io/crates/v/egui-android-ui)](https://crates.io/crates/egui-android-ui) | Виджеты (Button, Text, Spacer, Icon), контейнеры (Column, Row, Stack, LazyColumn), модификаторы, remember, анимации, темы Material Design 3 |
| egui-android-runtime | [![crates.io](https://img.shields.io/crates/v/egui-android-runtime)](https://crates.io/crates/egui-android-runtime) | Application, Dispatcher, StateStore, UiNotifier, AppConfig |
| egui-android-navigation | [![crates.io](https://img.shields.io/crates/v/egui-android-navigation)](https://crates.io/crates/egui-android-navigation) | ChildStack с управлением жизненным циклом |
| egui-android-platform | [![crates.io](https://img.shields.io/crates/v/egui-android-platform)](https://crates.io/crates/egui-android-platform) | Абстрактный Platform trait, FrameInput, PlatformEvent |
| egui-android-platform-android | [![crates.io](https://img.shields.io/crates/v/egui-android-platform-android)](https://crates.io/crates/egui-android-platform-android) | Android: EGL, input, главный цикл, lifecycle, system bars |
| egui-android-framework | [![crates.io](https://img.shields.io/crates/v/egui-android-framework)](https://crates.io/crates/egui-android-framework) | Umbrella, re-export всего |

## Технологии

| Зависимость | Версия | Назначение |
|---|---|---|
| egui | 0.35 | GUI |
| egui_glow | 0.35 | OpenGL рендеринг |
| glow | 0.17 | OpenGL context |
| android-activity | 0.6 | GameActivity (GL-режим) |
| ndk | 0.9 | NDK bindings |
| tokio (sync) | 1 | watch-каналы для StateStore |
| androidx.games:games-activity | 4.4.0 | GameActivity (Java/Kotlin) |
| androidx.appcompat | 1.6.1 | AppCompat тема для GameActivity |

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
┌──────────────┐   ┌──────────────┐   ┌─────────────────┐
│  Navigation  │   │     UI       │   │      Core       │
│  ChildStack  │   │  Виджеты     │   │  Component      │
│  Lifecycle   │   │  Контейнеры  │   │  Widget<M>      │
│              │   │  Модификаторы│   │  BackDispatcher │
│              │   │  Анимации    │   │  ComponentCtx   │
│              │   │  Тема        │   │  UiWrapper      │
└──────┬───────┘   └──────┬───────┘   └────────┬────────┘
       │                  │                    │
       └──────────────────┼────────────────────┘
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
                             │ Lifecycle         │
                             │ System bars       │
                             └──────────────────┘
```

### Поток данных (MVI + реактивное состояние)

```
UI (нажатие кнопки)
  → dispatch.dispatch(Msg::Increment)
    → Receiver накапливает сообщения
      ← после render: drain receiver
        → Component::handle(msg) — команда в data layer
          → Data Layer → store.update(|s| s.count += 1)
            → notify_tx.send(()) — сигнал Runtime
              → UiNotifier::check() → request_repaint() + waker.wake()
                → frame() → render(state, &dispatcher) → новый UI
```

## Лицензия

MIT или Apache 2.0
