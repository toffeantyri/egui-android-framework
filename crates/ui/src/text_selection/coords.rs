//! Стабилизация координат выделения внутри скролл-контейнеров.
//!
//! Проблема: `egui::ScrollArea` сдвигает контентный `Ui` на `-offset` относительно
//! мировых координат (`content_max_rect.min = inner_rect.min - state.offset`). Из-за
//! этого локальный rect виджета (`hit_rect`, построенный из `galley_pos`) и мировой
//! указатель (`pointer.latest_pos()`) живут в разных системах координат, и long-press
//! на прокрученной части текст не распознаётся.
//!
//! Решение: накопленная по цепочке контейнеров трансляция `total_shift` переводит
//! локальный rect в мировые координаты (или, симметрично, мировой указатель — в
//! локальные), где сравнение совпадает при любой глубине вложенности (в т.ч. для
//! вложенных `ScrollArea`).
//!
//! Чистая математика — без egui-рендера, тестируется на хосте.

use egui::{Pos2, Rect, Vec2};

/// Значение накопленной трансляции при отсутствии пролиста/контейнеров.
pub const NO_SHIFT: Vec2 = Vec2::ZERO;

/// Привести локальный `rect` (в координатах контента `ScrollArea`) к мировым,
/// прибавив накопленный сдвиг цепочки контейнеров.
///
/// Инвариант: мировой пиксель `P` лежит в локальном контенте по координате
/// `P - total_shift`, поэтому `rect_local` виден в мире как `rect_local + total_shift`.
pub fn local_rect_to_world(rect_local: Rect, total_shift: Vec2) -> Rect {
    rect_local.translate(total_shift)
}

/// Проверить, попадает ли мировой указатель в локальный `rect` виджета,
/// зная накопленную трансляцию цепочки скролл-контейнеров.
pub fn world_pointer_in_local_rect(
    pointer_world: Pos2,
    rect_local: Rect,
    total_shift: Vec2,
) -> bool {
    local_rect_to_world(rect_local, total_shift).contains(pointer_world)
}

/// Накопленный сдвиг для вложенных контейнеров: складывается по цепочке.
pub fn accumulate(parent_shift: Vec2, child_shift: Vec2) -> Vec2 {
    parent_shift + child_shift
}

/// Включить guard «приостановить скролл на время распознавания long-press».
///
/// Патч `egui::containers::scroll_area` читает время старта и не двигает контент,
/// пока оно моложе `LONG_PRESS_GUARD_SEC`. Вызывается при новом нажатии (`down`)
/// на selectable-тексте; снимается при распознанном выделении или отпускании.
/// Таймаут (форс-снятие) держит патч даже если снимут позже.
/// Константа должна совпадать с таймаутом в патче `scroll_area.rs`.
pub const LONG_PRESS_GUARD_SEC: f64 = 0.45;

pub fn set_long_press_guard(ctx: &egui::Context, now: f64) {
    ctx.data_mut(|d| {
        d.insert_persisted::<f64>(egui::containers::scroll_area::long_press_guard_id(), now)
    });
}

pub fn clear_long_press_guard(ctx: &egui::Context) {
    ctx.data_mut(|d| d.remove::<f64>(egui::containers::scroll_area::long_press_guard_id()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::vec2;

    #[test]
    fn local_rect_to_world_without_shift_identity() {
        let r = Rect::from_min_max(Pos2::new(10.0, 20.0), Pos2::new(30.0, 40.0));
        // Без скролла локальные == мировые.
        assert_eq!(local_rect_to_world(r, NO_SHIFT), r);
    }

    #[test]
    fn world_pointer_in_local_rect_translates_position() {
        let rect_local = Rect::from_min_max(Pos2::new(10.0, 20.0), Pos2::new(30.0, 40.0));
        // Сдвиг контента вниз/вправо на (100, 50): мировой rect смещён туда же.
        let shift = vec2(100.0, 50.0);
        let world_inside = Pos2::new(110.0, 70.0); // локально (10,20)
        let world_outside = Pos2::new(50.0, 30.0); // локально (-50,-20)
        assert!(world_pointer_in_local_rect(world_inside, rect_local, shift));
        assert!(!world_pointer_in_local_rect(
            world_outside,
            rect_local,
            shift
        ));
    }

    #[test]
    fn accumulate_stack_for_nested_scrolls() {
        // Вложенные ScrollArea: сдвиги складываются по цепочке.
        let outer = vec2(0.0, 120.0);
        let inner = vec2(0.0, 40.0);
        assert_eq!(accumulate(outer, inner), vec2(0.0, 160.0));
        // Три уровня.
        assert_eq!(
            accumulate(accumulate(outer, inner), vec2(0.0, 10.0)),
            vec2(0.0, 170.0)
        );
    }

    #[test]
    fn zero_shift_does_not_change_hit_test() {
        let rect_local = Rect::from_min_max(Pos2::ZERO, Pos2::new(5.0, 5.0));
        assert!(world_pointer_in_local_rect(
            Pos2::new(2.0, 2.0),
            rect_local,
            NO_SHIFT
        ));
        assert!(!world_pointer_in_local_rect(
            Pos2::new(9.0, 9.0),
            rect_local,
            NO_SHIFT
        ));
    }
}
