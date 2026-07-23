# JNI-мост для kill/restore процесса

## 📋 Оценка задачи

**Глобально (зачем):**

- **Пользователь продукта:** переживает убийство процесса системой без потери
  навигации и данных. Повернул экран — остался на том же экране с теми же
  данными. Система убила приложение — после перезапуска вернулся туда же.
  Как в нативном Android-приложении.

- **Разработчик фреймворка:** получает готовый инфраструктурный слой JNI,
  который автоматически передаёт SavedStack между Rust и Android Bundle.
  Никакого ручного управления Bundle на уровне приложения. Пишет компонент
  с PersistentState — и save/restore работает и при повороте, и при kill.

**Задача:** реализовать JNI-мост, передающий сериализованный Vec<u8>
(SavedStack<C>) между Rust PlatformState и Android Bundle через
kotlin-бridge в EguiActivity.

### Критерии успеха

- [ ] При повороте экрана навигация и состояние сохраняются (config change)
- [ ] При kill процесса + перезапуске навигация и состояние восстанавливаются
- [ ] JNI-функции корректно получают/устанавливают byteArray из Kotlin
- [ ] PlatformState не хранит бизнес-логику — только буфер raw bytes
- [ ] Логи верифицируют весь путь: on_save_state → JNI → Bundle → JNI → on_restore_state

### Границы (не делаем)

- ❌ Не меняем механизм save/restore внутри ChildStack — он уже работает
- ❌ Не добавляем новые поля в SavedStack или ComponentNode
- ❌ Не меняем Application trait — сигнатуры уже готовы
- ❌ Не трогаем тесты навигации (12 тестов, все проходят)

---

## План выполнения

| # | Шаг | Инвазивность | Файлы | Тесты |
|---|-----|--------------|-------|-------|
| 1 | Rust: PlatformState buffer | 🟠 Средняя | `platform_state.rs` (1 файл) | юнит: set_saved_state/take_saved_state |
| 2 | Rust: JNI-функции | 🟠 Средняя | `saved_state_jni.rs` (новый), `lib.rs` | — (тест только на Android) |
| 3 | Rust: Lifecycle связка | 🟠 Средняя | `lifecycle.rs` (1 файл) | юнит: handle_stop/destroy передают в buffer |
| 4 | Rust: Главный цикл связка | 🟡 Низкая | `loop.rs` (1 файл) | — (проброс параметра) |
| 5 | Kotlin: EguiActivity | 🟡 Низкая | 3 файла `.kt` | — (тест только на Android) |
| 6 | Application: проверка | 🟢 Косметика | `app.rs` | — (без изменений, проверка) |
| 7 | Тестирование на устройстве | 🟢 Проверка | APK сборка | adb kill + перезапуск + логи |
| 8 | Документация | 🟢 Косметика | 2 файла SKILL.md | — |

**Порядок:** от инвазивных к косметике. PlatformState создаётся первым — на нём
завязаны JNI-функции и lifecycle. Kotlin идёт после JNI-функций, т.к. зависит
от наличия nativeGetSavedState/nativeSetSavedState.

---

## Пошаговое выполнение

### Шаг 1. PlatformState buffer

Добавить `saved_state_buffer: Option<Vec<u8>>` в `PlatformStateInner`.
Методы `set_saved_state(bytes)` / `take_saved_state() -> Option<Vec<u8>>`.
Глобальный `OnceLock<PlatformState>` для доступа из JNI (UI thread).

**Тесты:** проверить, что set + take работают, take очищает буфер.

### Шаг 2. JNI-функции

Новый файл `saved_state_jni.rs` с `#[no_mangle]` функциями:
- `nativeGetSavedState` — берёт bytes из глобального PlatformState, возвращает jbyteArray
- `nativeSetSavedState` — читает jbyteArray, кладёт в глобальный PlatformState

**Тесты:** невозможны на хост-системе (нужен Android). Проверка через логи на устройстве.

### Шаг 3. Lifecycle связка

`handle_stop`/`handle_destroy`:
  - результат `on_save_state()` → `platform_state.set_saved_state()`

`handle_init_window`:
  - `platform_state.take_saved_state()` → `app.on_restore_state()`

Добавить `&PlatformState` параметр во все lifecycle-функции.

**Тесты:** юнит на handle_stop (передаёт bytes в buffer).

### Шаг 4. Главный цикл связка

`tick()` в `loop.rs` — пробросить `&PlatformState` из backend в `handle_lifecycle_event`.

### Шаг 5. Kotlin EguiActivity

Добавить в 3 файла:
- `nativeGetSavedState(): ByteArray?`
- `nativeSetSavedState(bytes: ByteArray?)`
- `onSaveInstanceState` → вызывает nativeGet, сохраняет в Bundle
- `onCreate` → читает из Bundle, вызывает nativeSet

### Шаг 6. Application проверка

`ShowcaseApplication` — проверить, что `on_restore_state(Some(bytes))` и
`on_save_state()` уже работают корректно. Изменений не требуется.

### Шаг 7. Тестирование на устройстве

1. Собрать showcase APK при помощи скрипта: `.\examples\showcase\run_android.sh`
2. Запустить, перейти на State Screen, изменить counter на 5
3. Повернуть экран → проверить логи: counter = 5
4. `adb shell am kill com.example.egui_android`
5. Перезапустить → проверить логи: counter = 5

### Шаг 8. Документация

Обновить `android-egui-architecture/SKILL.md` (JNI-мост, kill/restore поток).
Обновить `egui-android-guide/SKILL.md` (как работает JNI-мост).

---

## Риски

| Риск | Вероятность | План B |
|------|-------------|--------|
| `OnceLock` нестабилен для Android NDK | Низкая | Использовать `std::sync::Mutex<Option<PlatformState>>` |
| JNI имена не совпадают с Kotlin package | Средняя | Проверить `nativeGetSavedState` через `nm -D lib.so` |
| `onSaveInstanceState` не вызывается при configChanges | Низкая | Проверить AndroidManifest, убрать configChanges если нужно |
| `android-activity` не даёт доступ к Bundle | Средняя | Передавать bytes через отдельный static метод в Activity |

---

## Быстрые проверки перед каждым шагом

```
Шаг 1: PlatformState.set(Some(vec![1,2,3])); assert!(take() == Some(vec![1,2,3])); assert!(take() == None);
Шаг 3: handle_stop(app, &ps) → ps.take_saved_state() == Some(bytes)
Шаг 7: adb logcat | grep "on_save_state\|on_restore_state\|saved_state"
```
