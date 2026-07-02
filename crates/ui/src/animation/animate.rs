use std::hash::Hash;

pub fn animate_value(ui: &egui::Ui, key: impl Hash, target: f32, duration_secs: f32) -> f32 {
    let id = ui.id().with(key);
    ui.ctx().animate_value_with_time(id, target, duration_secs)
}

pub fn animate_bool(ui: &egui::Ui, key: impl Hash, target: bool, duration_secs: f32) -> f32 {
    let id = ui.id().with(key);
    ui.ctx().animate_bool_with_time(id, target, duration_secs)
}
