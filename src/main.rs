mod app;
mod buffer;
mod compiler;
mod completions;
mod lexer;
mod preview;
mod types;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 860.0])
            .with_title("LaTeX Writer"),
        ..Default::default()
    };

    eframe::run_native(
        "LaTeX Writer",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
