# Синхронизация IME-сервиса с буфером поля

> Дата: фикс «ввод в середину текста» (вариант B).
> Крейты: `egui-android-platform-android` (`loop.rs`, `ime_service.rs`), `egui-android-ui` (`text_edit.rs`), `egui-android-runtime` (`ImeEditorState`).

## Проблема

`ime_service` (`DefaultImeService`) — редьюсер IME-команд. Он ведёт собственное
состояние: `text_snapshot`, `cursor`, `composition_range`, `batch_depth`.

До фикса это состояние в рантайме **никогда не синхронизировалось** с реальным
текстом и кареткой поля egui:

- `UiCmd::SyncText` / `UiCmd::MoveCursor` определены, но из `loop.rs` не подавались;
- UI публикует текст+каретку в `PlatformState.ime_editor_state` (структура
  `ImeEditorState`), но `ime_service` об этом не знал.

Следствие — «фантомный снапшот»: `Region` / `DeleteSurrounding` / коммиты с
живым предиктом считали позиции по устаревшим данным. Признаки бага на
устройстве:

- тап в середину слова «привет» + ввод → текст попадал в конец или затирал слово;
- Gboard при тапе **не шлёт `setSelection`**, а работает через
  `getSelectionStart`/`getTextBeforeCursor` + `setComposingRegion`/`setComposingText`.

## Решение (вариант B — инкрементальный)

В `loop.rs` ПОСЛЕ `app_instance.frame(...)` (когда egui уже применил события и
обновил каретку) читаем `ImeEditorState` из `PlatformState` и подаём его в
`ime_service` через новую хост-функцию `sync_from_editor_state`:

```
frame()                          # egui обновил буфер и каретку
  └─ ImeEditorState              # UI опубликовал текст + selection (UTF-16)
       └─ sync_from_editor_state(service, text, selection_start_utf16)
            ├─ UiCmd::SyncText(text)
            └─ UiCmd::MoveCursor(cursor_char)   ← utf16 → char
```

`defaultImeService.apply(UiCmd::MoveCursor)` уже сбрасывает `composition_range`
и ставит `cursor` — живой регион старой композиции не «перекрывает» новый ввод.

**Однокадровая задержка синхронизации приемлема:** IME асинхронен; к следующему
`setComposingText`/`Region` сервис уже синхронизирован.

### ⚠ Регрессия варианта B: дубль при живой композиции (исправлено)

Изначальный вариант B вызывал `UiCmd::MoveCursor` безусловно. Это сломало
**непрерывное наращивание слова с живой композицией**: `MoveCursor` сбрасывает
`composition_range`, и следующий `Composing` начинал с `Insert` вместо `Replace`
→ слово дублировалось (лог устройства: после ввода «ппривет» следующий
`Composing("пприве")` шёл как Insert в конец → «пприветппривет»).

**Фикс:** `sync_from_editor_state` подаёт `MoveCursor` ТОЛЬКО когда в сервисе
нет живой композиции (`state().composition_range == None`):

```
frame()
  └─ sync_from_editor_state(service, text, selection_start_utf16)
       ├─ UiCmd::SyncText(text)        # всегда — держит снапшот актуальным
       └─ UiCmd::MoveCursor(cursor)    # только если composition_range == None
```

Синхронизация каретки важна при СТАТИЧЕСКОМ тексте (тап в середину, ввода нет).
При активной композиции каретку уже обновляют `Region`/`Composing`, трогать её
не нужно. Покрыто регресс-тестом `sync_between_growth_does_not_duplicate`.

### Хост-функция

```rust
// crates/platform-android/src/ime_service.rs
pub fn sync_from_editor_state(
    service: &mut dyn ImeService,
    text: &str,
    selection_start_utf16: usize,
) {
    // 1. Снапшот — всегда: должен отражать реальный буфер для последующих
    //    Region/Commit/DeleteSurrounding.
    service.apply(ImeCommand::Ui(UiCmd::SyncText(text.to_owned())));

    // 2. Каретка — только вне живой композиции, чтобы не разорвать
    //    наращивание preedit (иначе следующий Composing идёт как Insert → дубль).
    if service.state().composition_range.is_none() {
        let cursor = crate::ime_logic::utf16_offset_to_char_index(text, selection_start_utf16);
        service.apply(ImeCommand::Ui(UiCmd::MoveCursor(cursor)));
    }
}
```

Хост-совместимая (без `cfg(android)`): покрыта юнит-тестом
`sync_from_editor_state_updates_snapshot_and_cursor` и может вызываться откуда
угодно.

## Дополнительно: `SetSelection` как подстраховка

В `translate_legacy` команда `SetSelection` раньше игнорировалась. Теперь она
транслируется в `UiCmd::MoveCursor`:

```rust
Leg::SetSelection { start, .. } => {
    return Some(ImeCommand::Ui(UiCmd::MoveCursor((*start).max(0) as usize)));
}
```

Это подстраховка для IME, которые при тапе шлют `setSelection` (Gboard — нет,
но другие клавиатуры могут). `start` трактуется как char-индекс (после
конвертации UTF-16→char в JNI-слое). Для non-BMP (эмодзи) возможна неточность
на раредеких позициях — известное ограничение.

## Почему вариант B, а не A

| | Вариант A (сервис не владеет текстом) | Вариант B (синк из loop.rs) |
|---|---|---|
| Контракт `ImeService::apply` | Меняется (текст параметром) | Не меняется |
| Хост-тесты снапшота (`invariant_snapshot_matches_buffer` и др.) | Нужно переписывать | Остаются зелёными |
| Инвазивность | 🔴 высокая | 🟠 средняя |
| Архитектурная чистота | выше (единый источник) | ниже (редьюсер хранит снапшот) |

Вариант A отложен как возможный рефактор, если B окажется недостаточным (например,
при новой IME или более глубоких рассинхронах).

## Границы

- ❌ Не меняем контракт `ImeService::apply` (вариант A отложен).
- ❌ Не ломаем хост-совместимость `ime_service` (без `cfg(android)`).
- ❌ `sync_from_editor_state` не читает JNI — только примитивы (`text`, `selection`).
- ❌ JNI-конвертация `setSelection` как «основной» путь — не внедрялась (Gboard
  его не шлёт); `setSelection→MoveCursor` в `translate_legacy` — только подстраховка.

## Где что живёт

| Файл | Роль |
|---|---|
| `crates/ui/src/widgets/text_edit.rs` | UI публикует реальную каретку в `ImeEditorState` (`keyboard_publish_editor_state`) |
| `crates/runtime/src/keyboard_controller.rs` | `ImeEditorState` / `ImeEditorStateSlot` — общий слот UI→платформа |
| `crates/platform-android/src/platform_state.rs` | `ime_editor_state` — слот в `PlatformState` |
| `crates/platform-android/src/loop.rs` | после `frame()` вызывает `sync_from_editor_state` |
| `crates/platform-android/src/ime_service.rs` | `sync_from_editor_state`, `UiCmd`, `translate_legacy` (SetSelection→MoveCursor) |
| `crates/platform-android/src/ime_logic.rs` | `utf16_offset_to_char_index` (UTF-16→char) |
| `examples/showcase/src/ime_tests.rs` | device-тесты: вставка в середину (один и много символов) |

## Тесты

**Host:**
- `ime_service::sync_from_editor_state_updates_snapshot_and_cursor`
- `ime_service::sync_between_growth_does_not_duplicate` (регрессия дубля при синхронизации)
- `ime_service::insert_mid_word_preserves_prefix_via_sync_and_move`
- `ime_service::translate_legacy_maps_commands` (SetSelection→MoveCursor)

**Device** (`examples/showcase`, режим «только тесты»):
- `insert_mid_word_preserves_prefix` — «привет» + каретка в 3 + ввод «X» → «приXвет»
- `insert_multi_char_mid_word` — то же, но несколько символов («XL») → «приXLвет»

## Известные ограничения

1. **`text_snapshot` сервиса** всё ещё хранится в сервисе (не убран). Синхронизация
   через `sync_from_editor_state` держит его актуальным к моменту обработки команд,
   но двойное хранилище остаётся. Полное устранение — вариант A.
2. **Non-BMP/эмодзи** и конвертация UTF-16→char в `setSelection`-подстраховке — точны
   только при наличии текста; в JNI-слое конвертация выполняется через
   `utf16_offset_to_char_index`.
3. Синхронизация происходит раз в кадр; при очень быстрых двух вводах в одном кадре
   второй может использовать ещё не обновлённый снапшот — на практике не наблюдалось.
4. Во время АКТИВНОЙ композиции `sync_from_editor_state` намеренно НЕ двигает каретку
   (`MoveCursor` пропускается). Если сам Gboard передумает и сместит каретку внутри
   предикта без `setComposingRegion`, сервис не подхватит это до конца композиции —
   корректный канал для этого и есть `Region`/`Composing`, что соответствует реальному
   протоколу Gboard.
