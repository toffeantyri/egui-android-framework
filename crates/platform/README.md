# egui-android-platform

**Платформенная абстракция: Waker (пробуждение event loop) и SystemTheme (Light/Dark).**

Конкретная реализация — в `egui-android-platform-android`.

[![crates.io](https://img.shields.io/crates/v/egui-android-platform)](https://crates.io/crates/egui-android-platform)

## Состав

### Waker
- Пробуждение event loop платформы
- Обёртка над замыканием (`AndroidApp::signal()` для Android)
- Используется RuntimeContext для уведомления платформы

### SystemTheme
- Системная тема: Light / Dark
- Используется в platform-android для настройки системных баров

## Когда использовать

Подключайте `egui-android-platform`, если вы:
- пишете свою платформенную реализацию (нужен Waker)
- работаете с системной темой на уровне платформы
