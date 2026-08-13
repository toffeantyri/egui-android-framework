# Патч egui 0.35.0: Fix scroll jump on touch after fling

## Проблема
При повторном касании после fling-анимации ScrollArea мгновенно проскакивает
на 100-200pt. Дёрганье происходит даже после полной остановки инерции.

## Корень
`pointer.delta()` на первом кадре нового drag вычисляется как 
`new_pos - latest_pos_from_previous_gesture`. Поскольку latest_pos остаётся
от предыдущего UP, разница может составлять сотни пикселей.

Дополнительно: остаточная `state.vel` от fling применяется в первом кадре
нового drag, усугубляя скачок.

## Исправления

### 1. input_state/mod.rs — обнуление old_pos при Down
В `PointerState::begin_pass()` после цикла обработки событий, перед вычислением
`self.delta`, добавлена проверка:
```rust
if self.any_pressed() && !self.any_released() {
    old_pos = self.latest_pos;
}
```
Гарантирует `pointer.delta() == Vec2::ZERO` на первом кадре нового drag.
Переменная `old_pos` сделана мутабельной (`let mut old_pos`).

### 2. scroll_area.rs — сброс vel при drag_started
При `drag_started()` обнуляем остаточную скорость:
```rust
state.vel = Vec2::ZERO;
```

### 3. platform-android батчинг (вне egui, в platform-android)
В кадр с Down попадает ТОЛЬКО `PointerButton { pressed: true }`.
Все Move откладываются на следующий кадр через deferred_events.
Это дополнительная гарантия, что первый Move видит корректный old_pos.

## Апстрим
Issue: не создавался
PR: не создавался
Статус: локальный патч для egui-android-framework

## Удаление патча
Когда egui >= X.Y.Z с фиксом будет выпущен:
1. Удалить `[patch.crates-io] egui` из корневого Cargo.toml
2. Удалить директорию patches/egui/
3. Обновить версию egui в Cargo.toml
4. Откатить batсhing-логику в `crates/platform-android/src/run.rs`

## Проверка
- [x] `cargo check --workspace` — без ошибок

---

# IME: каретка в конец после preedit (replace_range)

## Проблема
Пользователь вводил «привет как дела» — после пробела каждое новое слово
заменяло предыдущее (в поле оставался только конец).

## Корень
После `ImeEvent::Preedit` с `replace_range` egui сохранял не каретку в конце
вставки, а `CCursorRange::two(start, end)` (выделение диапазона 0..N).
Следующая вставка (новое слово) заменяла выделение → набранное стиралось.

## Исправление
`patches/egui/src/widgets/text_edit/builder.rs` — в ветке `Preedit` с
`replace_range` возвращаем `CCursorRange::one(ccursor)` (каретка в конце), а не
`two(start, end)`. Следующая вставка теперь дописывает в конец.

## Проверка
- Регрессионный тест `preedit_replace_range_then_insert_appends_not_overwrites`
  (crates/ui) — RED «к вместо привет к» → GREEN.
- На устройстве «привет как дела» накапливается целиком.

---

# IME: мигающая каретка и выделение при активном предикте (legacy_visuals)

## Проблема
После ввода «р» каретка переставала мигать; текст не выделялся.

## Корень
Непустой `ImeEvent::Preedit` переводит `cursor_purpose` в `ImeComposition`: в
`show()` мигающая каретка и выделение рисуются ТОЛЬКО при `Selection`; при
`ImeComposition` рисуется подчёркивание предикта, которого на устройстве нет.
`legacy_visuals` по умолчанию был `cfg!(windows)`, поэтому на Android поле
оставалось без каретки/выделения при предикте.

## Исправление
`patches/egui/src/style.rs` — `default_legacy_visuals()` теперь `true` и на
Android: `cfg!(any(target_os = "windows", target_os = "android"))`. В legacy-режиме
предикт рисуется как обычное выделение, мигающая каретка всегда в конце.

## Проверка
- Device-тест `ime_caret_visible_uses_legacy_visuals` (на устройстве проверяет
  `visuals.ime_composition.legacy_visuals == true`).
- На устройстве каретка мигает при вводе.
