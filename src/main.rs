#![windows_subsystem = "windows"]

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
        Box::new(|cc| {
            let mut style = egui::Style::default();
            style.visuals = egui::Visuals::dark();
            let accent = egui::Color32::from_rgb(180, 180, 180);
            style.visuals.selection.bg_fill = accent;
            style.visuals.panel_fill = egui::Color32::from_rgb(20, 20, 25);
            style.visuals.window_fill = egui::Color32::from_rgb(25, 25, 30);
            style.visuals.window_corner_radius = egui::CornerRadius::same(12);
            style.visuals.menu_corner_radius = egui::CornerRadius::same(8);
            style.visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(8);
            style.visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(8);
            style.visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(8);
            style.visuals.widgets.active.corner_radius = egui::CornerRadius::same(8);
            style.visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(30, 30, 35);
            style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(45, 45, 55);
            style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(65, 65, 80);
            cc.egui_ctx.set_global_style(style);
            Ok(Box::new(app::App::new(cc)))
        }),
    )
}
