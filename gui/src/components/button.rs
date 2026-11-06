use egui::{Button, Color32, WidgetText};

/// A standard button.
pub fn standard<'a>(text: impl Into<WidgetText>) -> Button<'a> {
    Button::new(text)
}

/// A button styled for the toolbar, with specific minimum size and fill color.
pub fn toolbar<'a>(text: impl Into<WidgetText>, fill: Color32) -> Button<'a> {
    Button::new(text)
        .min_size(egui::vec2(90.0, 26.0))
        .fill(fill)
}

/// A button styled for the toolbar without a specific background fill color.
#[allow(dead_code)]
pub fn toolbar_standard<'a>(text: impl Into<WidgetText>) -> Button<'a> {
    Button::new(text).min_size(egui::vec2(90.0, 26.0))
}

/// A square button typically used for icons in panels (like the preview panel).
pub fn icon<'a>(text: impl Into<WidgetText>) -> Button<'a> {
    Button::new(text)
}

/// A square button with a specific size.
#[allow(dead_code)]
pub fn icon_sized<'a>(text: impl Into<WidgetText>, size: egui::Vec2) -> Button<'a> {
    Button::new(text).min_size(size)
}

/// A borderless, transparent button, often used for close 'x' buttons.
pub fn borderless<'a>(text: impl Into<WidgetText>) -> Button<'a> {
    Button::new(text).frame(false)
}
