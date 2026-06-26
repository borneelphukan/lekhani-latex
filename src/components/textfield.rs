use egui::TextEdit;

/// A standard single-line text field.
#[allow(dead_code)]
pub fn singleline(text: &mut String) -> TextEdit<'_> {
    TextEdit::singleline(text)
}

/// A single-line text field styled as a password input.
pub fn password(text: &mut String) -> TextEdit<'_> {
    TextEdit::singleline(text)
        .password(true)
        .desired_width(f32::INFINITY)
}

/// A standard multi-line text field.
#[allow(dead_code)]
pub fn multiline(text: &mut String) -> TextEdit<'_> {
    TextEdit::multiline(text)
}
