use egui::{ComboBox, WidgetText};
use std::hash::Hash;

/// A standard dropdown (ComboBox).
pub fn dropdown(id_salt: impl Hash, selected_text: impl Into<WidgetText>) -> ComboBox {
    ComboBox::from_id_salt(id_salt).selected_text(selected_text)
}
