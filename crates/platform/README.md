# egui-android-platform

**Абстрактный Platform trait для egui на Android.**

Определяет контракт платформы: Platform, PlatformEvent, FrameInput, FrameOutput, PlatformConfig.
Конкретная реализация — в `egui-android-platform-android`.

[![crates.io](https://img.shields.io/crates/v/egui-android-platform)](https://crates.io/crates/egui-android-platform)

## Состав

### Platform (trait)
- Абстракция над платформой (Android, десктоп, web)

### PlatformEvent
- События: Touch, Key, BackPressed, Lifecycle (Resume, Pause, Destroy)

### FrameInput / FrameOutput
- Входные/выходные данные для одного кадра

### PlatformConfig
- Настройки платформы

## Когда использовать

Подключайте `egui-android-platform`, если вы:
- пишете свою платформенную реализацию
- используете абстракцию для тестирования на хосте
