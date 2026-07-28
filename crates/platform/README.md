# egui-android-platform

**Платформенная абстракция: Waker (пробуждение event loop) и SystemTheme (Light/Dark).**

Этот крейт определяет два базовых типа, которые конкретная платформа (Android, Desktop, Web) реализует по-своему.
Конкретная реализация для Android — в [`egui-android-platform-android`](https://crates.io/crates/egui-android-platform-android).

[![crates.io](https://img.shields.io/crates/v/egui-android-platform)](https://crates.io/crates/egui-android-platform)

## Состав

### `Waker`
- Пробуждение event loop платформы
- Обёртка над замыканием (на Android — `AndroidApp::signal()`)
- Используется `RuntimeContext` для уведомления платформы об изменении состояния

### `SystemTheme`
- Системная тема: Light / Dark
- Используется в platform-android для настройки цвета системных баров под тему

## Проблема

Рантайму нужно разбудить платформу, когда изменились данные (чтобы перерисовать UI).
На каждой платформе это делается по-разному. `Waker` — единый интерфейс для этого.
Аналогично, тема системы (светлая/тёмная) определяется платформой — `SystemTheme` стандартизирует это.

## Когда использовать

Подключайте `egui-android-platform`, если вы:
- пишете свою платформенную реализацию (нужен `Waker`)
- работаете с системной темой на уровне платформы

Для пользователей фреймворка все типа доступны через `<a href="https://crates.io/crates/egui-android-framework">`**`egui-android-framework`**`</a>` (umbrella-крейт):

```rust
use egui_android::platform::{Waker, SystemTheme};
```

## Зависимости

Зависит от: `egui`

От него зависят: [`egui-android-platform-android`](https://crates.io/crates/egui-android-platform-android), [`egui-android-runtime`](https://crates.io/crates/egui-android-runtime)
