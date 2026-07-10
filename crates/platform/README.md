# egui-android-platform

Абстрактный Platform trait. Определяет контракт между платформой и фреймворком.

## Проблема

Android-специфичный код (EGL, NativeWindow, input) не должен быть жёстко завязан на фреймворк.
Этот крейт определяет абстрактный контракт, который реализует `egui-android-platform-android`.
В будущем возможно появление реализаций для других платформ (Linux, Windows).

## Состав

| Тип | Описание |
|---|---|
| **`Platform`** | Трейт для платформо-зависимой реализации (EGL, окно, ввод) |
| **`PlatformEvent`** | События платформы: InitWindow, Resume, Pause, Stop, Destroy |
| **`FrameInput` / `FrameOutput`** | Входные/выходные данные одного кадра |
| **`PlatformConfig`** | Конфигурация платформы (размеры, плотность пикселей) |

## Реализации

- **Android:** `egui-android-platform-android` — EGL, NDK input, главный цикл

## Зависимости

Нет внешних зависимостей.
