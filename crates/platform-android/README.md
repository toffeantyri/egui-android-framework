# egui-android-platform-android

**Реализация Platform для Android: EGL, input, главный цикл, lifecycle, system bars.**

Связывает `GameActivity`, EGL и `egui_glow` в единый цикл. Обрабатывает Android lifecycle,
touch-ввод (с инерцией), системные панели (status bar, navigation bar) и кнопку Back.

[![crates.io](https://img.shields.io/crates/v/egui-android-platform-android)](https://crates.io/crates/egui-android-platform-android)

## Проблема

Запустить egui на Android — нетривиальная задача:
- Нужно инициализировать EGL/OpenGL
- Интегрироваться с `GameActivity` lifecycle
- Пробросить touch-события в egui
- Обработать системные панели, insets, кнопку Back
- Всё это должно работать в одном главном цикле

Этот крейт решает все эти проблемы. Просто вызовите `run::<MyApplication>(app)`.

## Возможности

- **Главный цикл** — EGL + `poll_events()` + `egui_glow` + lifecycle. Оркестрация через `run.rs` и `RunState::tick()`
- **Touch-ввод** — `MotionEvent` → `egui::Event`. Скролл с инерцией (fling). Батчинг событий для исключения скачков scroll_offset
- **Кнопка Back** — перехват `AKEYCODE_BACK`, делегирование в `Application::on_back_pressed()`
- **Системные панели** — установка цвета status bar / navigation bar через JNI, корректировка insets для MIUI
- **Lifecycle** — InitWindow → Resume → Pause → Stop → Destroy с пробросом в Application
- **GraphicsPipeline** — OpenGL рендеринг через `egui_glow`

## Пайплайн IME-ввода и InputConnection

Текстовый ввод идёт ТОЛЬКО через Kotlin `InputConnection` (не через `TextEvent`/`textInputState`).

**Поток:**

```text
EguiImeView.EguiImeInputConnection (Kotlin)
        │  commitText / setComposingText / setComposingRegion /
        │  deleteSurroundingText / beginBatchEdit / endBatchEdit / …
        ▼
ime_jni.rs (JNI-функции nativeOn*)
        │  кладут ImeCmd в PlatformState.ime_cmds (потокобезопасная очередь)
        ▼
loop.rs (шаг 2.5)
        │  take_ime_cmds → translate_legacy → DefaultImeService.apply → ImeEvent
        ▼
to_egui_event
        │  Insert→Event::Text, Replace→ImeEvent::Preedit{replace_range},
        │  Delete→клавиша; события идут в egui в текущем кадре
        ▼
egui::TextEdit (патч)
```

**Ключевые компоненты:**
- `ime_jni.rs` — JNI-мост: 
  очередь команд + `getTextBeforeCursor`/`getTextAfterCursor`/`getSelectionStart`/
  `getExtractedText` (читают `ImeEditorState`, публикуемый UI-слоем).
- `ime_logic.rs` — типы `ImeCmd` и хост-утилиты.
- `ime_service.rs` — редьюсер (`DefaultImeService`): единственный источник состояния
  композиции (`text_snapshot`, `cursor`, `composition_range`) → буферные операции.
- `loop.rs` — забирает очередь, применяет редьюсер, конвертирует в `egui::Event`.

**Поведение (как обычный EditText / Unity / Godot):**
- Предикт (`Composing`) наращивается через `Replace` региона; после снятия предикта
  следующий `Commit` делает `Insert` (не перезаписывает) — слово накапливается.
- Каретка в `ImeEditorState` публикуется в КОНЕЦ текста во время ввода, поэтому
  Gboard через `getTextBeforeCursor`/`getSelection` получает корректный хвост, и
  регионы не затирают набранное.
- egui-patch: после `ImeEvent::Preedit{replace_range}` каретка встаёт в конец вставки
  (`CCursorRange::one`), а не выделяет диапазон — следующее слово дописывается.
- egui `Visuals::ime_composition.legacy_visuals = true` на Android: мигающая каретка
  и выделение остаются видимыми даже при активной IME-композиции.


## Использование

```rust
use egui_android_platform_android::run;

struct MyApp;
// impl Application for MyApp { ... }

#[no_mangle]
fn android_main(app: android_activity::AndroidApp) {
    run::<MyApp>(app);
}
```

Через umbrella-крейт:

```rust
use egui_android::platform_android::run;
```

## Зависимости

- [`egui-android-platform`](https://crates.io/crates/egui-android-platform) — Waker, SystemTheme
- [`egui-android-runtime`](https://crates.io/crates/egui-android-runtime) — Application, RuntimeContext
- EGL через `glow`
- OpenGL через `egui_glow`
- JNI для системных панелей
- `ndk` для NativeWindow
