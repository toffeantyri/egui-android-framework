//! Входные и выходные данные кадра.
//!
//! [`FrameInput`] — данные, которые платформа передаёт в приложение для рендеринга кадра.
//! [`FrameOutput`] — данные, которые приложение возвращает платформе после рендеринга.

/// Входные данные для одного кадра.
pub struct FrameInput {
    /// Сырой ввод egui (события, screen_rect, pixels_per_point).
    pub raw_input: egui::RawInput,

    /// Время, прошедшее с предыдущего кадра (в секундах).
    pub delta_time: f32,
}

impl FrameInput {
    /// Создать новый FrameInput.
    pub fn new(raw_input: egui::RawInput, delta_time: f32) -> Self {
        Self {
            raw_input,
            delta_time,
        }
    }
}

/// Выходные данные после рендеринга кадра.
pub struct FrameOutput {
    /// Результат рендеринга egui (shapes, textures, etc.).
    pub inner: egui::FullOutput,
}

impl FrameOutput {
    /// Создать новый FrameOutput.
    pub fn new(inner: egui::FullOutput) -> Self {
        Self { inner }
    }
}
