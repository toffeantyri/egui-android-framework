//! «Текстовая поверхность», публикуемая текстовым виджетом для выделения.
//!
//! Модель «[контент публикует → модификатор рисует]» (аналог паттерна
//! `TextEdit → ImeEditorStateSlot`): текстовый виджет (`Text` и будущие) рисует
//! текст единообразно и публикует свою galley/позицию в общий слот, а
//! `Modifier::selectable(true)` читает её и ведёт selection-слой поверх.

use std::sync::{Arc, RwLock};

use egui::{Context, Id, Pos2, Ui};

/// Что текстовый виджет публикует для выделения.
#[derive(Clone)]
pub struct TextSurface {
    pub id: egui::Id,
    pub galley: Arc<egui::Galley>,
    pub galley_pos: Pos2,
    pub text: String,
}

/// Общий слот «активной текстовой поверхности» в `egui::Context::data`.
pub type TextSurfaceSlot = Arc<RwLock<Option<TextSurface>>>;

/// Ключ слота в `Context::data`.
fn surface_slot_id() -> Id {
    Id::new("egui_android_text_surface")
}

/// Получить слот (создав при первом обращении), как в `remember`.
fn get_or_init_slot(ctx: &Context) -> TextSurfaceSlot {
    let id = surface_slot_id();
    if let Some(slot) = ctx.data(|d| d.get_temp::<TextSurfaceSlot>(id)) {
        return slot;
    }
    let slot: TextSurfaceSlot = Arc::new(RwLock::new(None));
    ctx.data_mut(|d| d.insert_temp(id, Arc::clone(&slot)));
    slot
}

/// Опубликовать (перезаписать) активную текстовую поверхность.
pub fn publish_text_surface(ui: &Ui, surface: TextSurface) {
    let slot = get_or_init_slot(ui.ctx());
    let mut guard = slot.write().expect("text surface slot poisoned");
    *guard = Some(surface);
}

/// Прочитать последнюю опубликованную поверхность (не удаляя её).
pub(crate) fn take_last_published_surface(ctx: &Context) -> Option<TextSurface> {
    let id = surface_slot_id();
    let slot: TextSurfaceSlot = ctx.data(|d| d.get_temp::<TextSurfaceSlot>(id))?;
    let guard = slot.read().ok()?;
    guard.clone()
}
