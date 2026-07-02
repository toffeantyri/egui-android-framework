//! Композиционные билдеры — аналоги `Column`, `Row`, `Box` из Jetpack Compose.
//!
//! Билдеры принимают замыкание, в котором рисуются дочерние виджеты.
//! Они не хранят состояние и не знают про `Dispatcher` — только про `egui::Ui`.
//!
//! # Пример
//!
//! ```ignore
//! column(ui, |ui| {
//!     ui.label("Первый");
//!     ui.label("Второй");
//! });
//! ```

/// Вертикальное расположение дочерних элементов.
///
/// Аналог `Column` в Jetpack Compose или `ui.vertical()` в egui.
///
/// # Пример
///
/// ```ignore
/// column(ui, |ui| {
///     ui.label("Строка 1");
///     ui.label("Строка 2");
/// });
/// ```
pub fn column(ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui)) {
    ui.vertical(content);
}

/// Горизонтальное расположение дочерних элементов.
///
/// Аналог `Row` в Jetpack Compose или `ui.horizontal()` в egui.
///
/// # Пример
///
/// ```ignore
/// row(ui, |ui| {
///     ui.label("Левый");
///     ui.label("Правый");
/// });
/// ```
pub fn row(ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(content);
}

/// Блок с отступами (аналог `Box` в Jetpack Compose).
///
/// Создаёт область с отступами со всех сторон.
///
/// # Пример
///
/// ```ignore
/// boxed(ui, 8.0, |ui| {
///     ui.label("Текст внутри блока");
/// });
/// ```
pub fn boxed(ui: &mut egui::Ui, padding: f32, content: impl FnOnce(&mut egui::Ui)) {
    ui.add_space(padding);
    ui.horizontal(|ui| {
        ui.add_space(padding);
        content(ui);
        ui.add_space(padding);
    });
    ui.add_space(padding);
}
