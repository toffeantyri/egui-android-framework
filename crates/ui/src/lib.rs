//! UI — композиционные билдеры, модификаторы, remember.
//!
//! Содержит:
//! - `remember` — UI-only состояние (hover, focus, scroll)
//! - `builders` — базовые композиционные билдеры (column, row, box)
//! - `modifier` — Modifier trait + расширения (padding, width, clickable)
//!
//! Этот крейт НЕ содержит готовых компонентов (Button, Text, Image).
//! НЕ знает про navigation.
