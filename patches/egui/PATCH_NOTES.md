# Патч egui 0.31.1: Fix scroll jump on touch after fling

## Проблема
При повторном касании после fling-анимации ScrollArea мгновенно проскакивает
на 100-200pt. Дёрганье происходит даже после полной остановки инерции.

## Корень
`pointer.delta()` на первом кадре нового drag вычисляется как 
`new_pos - latest_pos_from_previous_gesture`. Поскольку latest_pos остаётся
от предыдущего UP, разница может составлять сотни пикселей.

Дополнительно: остаточная `state.vel` от fling и `smooth_scroll_delta`
применяются в первом кадре нового drag, усугубляя скачок.

## Исправления

### 1. input_state/mod.rs — обнуление old_pos при Down
В `PointerState::begin_pass()` при обработке `PointerButton { pressed: true }`:
```rust
if pressed {
    old_pos = Some(pos); // устанавливаем old_pos = новой позиции
}
```
Гарантирует `pointer.delta() == Vec2::ZERO` на первом кадре нового drag.

**Изменение:**
```diff
+                    if pressed {
+                        // Обнуляем old_pos, чтобы pointer.delta()
+                        // на первом кадре нового drag была нулевой.
+                        old_pos = Some(pos);
                         self.pos_history.clear();
                     }
```

### 2. scroll_area.rs — сброс vel и smooth_scroll_delta
При `drag_started()` обнуляем остаточную скорость и плавный скролл:
```rust
state.vel[d] = 0.0;
ui.ctx().input_mut(|i| i.smooth_scroll_delta = Vec2::ZERO);
```

**Изменение:**
```diff
 if content_response_option
     .as_ref()
     .is_some_and(|response| response.drag_started())
 {
+    state.vel[d] = 0.0;
+    ui.ctx().input_mut(|input| {
+        input.smooth_scroll_delta = Vec2::ZERO;
+    });
 }
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
- [x] `cargo test --workspace` — все тесты проходят
- [x] Сценарий 1: Медленный скролл — плавный, без дёрганья
- [x] Сценарий 2: Быстрый fling — инерция работает
- [x] Сценарий 3: Касание ВО ВРЕМЯ fling — fling останавливается, нет скачка
- [x] Сценарий 4: Повторное касание ПОСЛЕ fling — нет дёрганья
- [x] Сценарий 5: Доскролл до краёв — контент достигает границ
- [x] Сценарий 6: Тап по кнопке — кнопка нажимается
- [x] Сценарий 7: Drag слайдера — работает без конфликтов
