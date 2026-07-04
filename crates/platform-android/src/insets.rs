//! Определение реальной высоты статус-бара.
//!
//! На MIUI `content_rect.top` возвращает завышенные значения (334px вместо 152px).
//! Используем эвристику: если `content_rect.top > 200px` — это MIUI с DisplayCutout,
//! реальный статус-бар ≈ 38pt (из mAppBounds: 152px / pp=4).

#![cfg(target_os = "android")]

use std::sync::Once;

static LOG_ONCE: Once = Once::new();

/// Получить высоту статус-бара в пикселях экранных координат (points).
///
/// - Если `content_rect.top` > 200px (MIUI workaround) — возвращает 38pt.
/// - Иначе — `content_rect.top / pp`.
pub fn get_top_inset_pt(native_height: i32, content_rect_top: i32, pp: f32) -> f32 {
    if content_rect_top > 200 {
        LOG_ONCE.call_once(|| {
            log::info!(
                "insets: MIUI workaround (content_rect.top={}px), используем 38pt (из mAppBounds: 152px / pp={})",
                content_rect_top, pp
            );
        });
        38.0
    } else {
        content_rect_top as f32 / pp
    }
}
