//! Android-подобное выделение текста для виджетов [`crate::widgets::Text`]
//! и [`crate::widgets::TextEdit`].
//!
//! Модель взаимодействия:
//! - длинное нажатие → выделение слова, drag-ручки, плавающий тулбар;
//! - общая логика — [`SelectionCore`] (`selection_core.rs`), per-widget через
//!   [`crate::remember::remember()`] (`Arc<RwLock<T>>` в `IdTypeMap`);
//! - рендер фона выделения ПОД глифами — через публичный
//!   `egui::text_selection::visuals::paint_text_selection` (`render.rs`);
//! - ручки/тулбар — поверх через `Area` + `Order::Foreground`
//!   (`drag_handles.rs`, `toolbar.rs`).
//!
//! Архитектурно изолирован от `platform-android`: не использует JNI, plugin-систему
//! egui (`LabelSelectionState`) и не изменяет патчи egui.
//! Модуль хост-совместим (без `cfg(target_os = "android")`) и покрыт юнит-тестами
//! на чистой логике (`cargo test -p egui-android-ui`).

pub mod android_behavior;
pub mod coords;
pub mod drag_handles;
pub mod render;
pub mod selection_core;
pub mod toolbar;

pub use android_behavior::{HandleSide, LongPressState};
pub use selection_core::{BufferCommand, SelectionCore};
pub use toolbar::ToolbarAction;
