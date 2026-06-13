mod app;
mod buffer;
mod compiler;
mod completions;
mod lexer;
mod preview;
mod types;

fn load_icon() -> Option<egui::IconData> {
    let img = image::open("assets/logo.png").ok()?;
    let img = img.into_rgba8();
    let (width, height) = img.dimensions();
    Some(egui::IconData {
        rgba: img.into_raw(),
        width,
        height,
    })
}

fn main() -> Result<(), eframe::Error> {
    let icon = load_icon();
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1280.0, 860.0])
        .with_title("Lekhani Latex");
    if let Some(icon) = icon {
        viewport = viewport.with_icon(std::sync::Arc::new(icon));
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Lekhani Latex",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
