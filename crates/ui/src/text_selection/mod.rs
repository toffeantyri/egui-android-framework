//! Android-подобное выделение статичного текста.
//!
//! Touch-специфичный слой для виджета [`crate::widgets::Text`]:
//! длинное нажатие → выделение слова, drag-ручки, плавающий тулбар.
//!
//! Архитектурно изолирован от `platform-android`: не использует JNI, plugin-систему
//! egui (`LabelSelectionState`) и не изменяет патчи egui. Рендер выделения — через
//! публичный `egui::text_selection::visuals::paint_text_selection`; состояние —
//! per-widget через `crate::remember::remember()` (`Arc<RwLock<T>>` в `IdTypeMap`).
//!
//! Модуль хост-совместим (без `cfg(target_os = "android")`) и покрыт юнит-тестами
//! на чистой логике (`cargo test -p egui-android-ui`).

pub mod android_behavior;
pub mod drag_handles;
pub mod selection_layer;
pub mod selection_toolbar;
pub mod state;
pub mod surface;

pub use android_behavior::{HandleSide, LongPressState};
pub use selection_toolbar::ToolbarAction;
pub use state::AndroidSelectionState;
pub use surface::{publish_text_surface, TextSurface};
