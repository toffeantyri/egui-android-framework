# egui-android-platform

Абстрактный Platform trait. Определяет контракт между платформой и фреймворком.

## Типы

- **`Platform`** — трейт для платформо-зависимой реализации
- **`PlatformEvent`** — события платформы (InitWindow, Resume, Pause, ...)
- **`FrameInput` / `FrameOutput`** — входные/выходные данные кадра
- **`PlatformConfig`** — конфигурация платформы

Используется в `platform-android` для Android-реализации.
