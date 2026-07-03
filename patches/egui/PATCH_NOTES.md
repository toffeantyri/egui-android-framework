# Патч egui 0.31.1: Fix ScrollArea velocity reset on drag start

## Проблема
При повторном касании после fling-анимации ScrollArea применяет остаточную 
velocity в первом кадре нового drag, вызывая мгновенный скачок scroll_offset 
(«дёрганье»).

ScrollArea хранит `state.vel` — остаточную скорость от fling. Когда пользователь 
начинает новый drag (новое касание), `state.vel` не обнуляется, и ScrollArea 
продолжает применять её в первом кадре drag, одновременно с `pointer.delta()`.
Это создаёт эффект «дёрганья».

## Исправление
В `src/containers/scroll_area.rs` добавлен сброс `state.vel[d] = 0.0` 
при `response.drag_started()` — когда пользователь начинает новое касание 
после fling.

**Изменение:**
```diff
 if content_response_option
     .as_ref()
     .is_some_and(|response| response.dragged())
 {
     for d in 0..2 {
         if scroll_enabled[d] {
+            // Сбрасываем остаточную скорость fling при начале нового drag.
+            if content_response_option
+                .as_ref()
+                .is_some_and(|response| response.drag_started())
+            {
+                state.vel[d] = 0.0;
+            }
             ui.input(|input| {
                 state.offset[d] -= input.pointer.delta()[d];
             });
```

## Апстрим
Статус: локальный патч для egui-android-framework

## Удаление патча
Когда egui >= X.Y.Z с фиксом будет выпущен:
1. Удалить `[patch.crates-io] egui` из корневого Cargo.toml
2. Удалить директорию patches/egui/
3. Обновить версию egui в Cargo.toml

## Проверка
- [x] `cargo check --workspace` — без ошибок
- [ ] Сценарий 1: Медленный скролл — плавный, без дёрганья
- [ ] Сценарий 2: Быстрый fling — инерция работает
- [ ] Сценарий 3: Касание ВО ВРЕМЯ fling — fling останавливается, нет скачка
- [ ] Сценарий 4: Повторное касание ПОСЛЕ fling — нет дёрганья
- [ ] Сценарий 5: Доскролл до краёв — контент достигает границ
- [ ] Сценарий 6: Тап по кнопке — кнопка нажимается
- [ ] Сценарий 7: Drag слайдера — работает без конфликтов
