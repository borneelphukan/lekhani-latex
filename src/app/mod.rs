pub mod tab;
mod toolbar;
mod menubar;
mod statusbar;
mod preview_panel;
mod editor;

use std::path::PathBuf;
use std::time::Instant;

use egui::{CentralPanel, Color32, Panel};
use std::sync::OnceLock;
use regex::Regex;

fn error_line_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"l\.(\d+)").unwrap())
}

use crate::compiler::CompileEvent;
use crate::preview::PreviewEvent;
use crate::types::*;

#[derive(Clone)]
enum UpdateState {
    None,
    Checking,
    Prompt(String),
    Downloading(f32),
}

enum UpdateMessage {
    CheckResult(bool, Option<String>),
    DownloadProgress(f32),
    DownloadComplete(Result<String, String>),
}

pub struct App {
    tabs: Vec<tab::Tab>,
    active_tab: usize,
    theme: Theme,
    auto_compile: bool,
    file_dialog_action: Option<FileDialogAction>,
    completion_visible: bool,
    completion_matches: Vec<String>,
    completion_byte_range: Option<(usize, usize)>,
    completion_selected: usize,
    completion_prefix: String,
    completion_block_trigger: bool,
    show_outputs: bool,
    show_outputs_requested: bool,
    about_open: bool,
    update_state: UpdateState,
    update_rx: std::sync::mpsc::Receiver<UpdateMessage>,
    update_tx: std::sync::mpsc::Sender<UpdateMessage>,
    heading_numbered: bool,
    last_auto_compile: Option<Instant>,
    last_content_change: Option<Instant>,
    tab_drag: Option<(usize, f32)>,
}

enum FileDialogAction {
    Open,
    Save,
    SaveAs,
    NewDocument,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (update_tx, update_rx) = std::sync::mpsc::channel::<UpdateMessage>();
        let mut app = Self {
            tabs: Vec::new(),
            active_tab: 0,
            theme: Theme::System,
            auto_compile: true,
            file_dialog_action: None,
            completion_visible: false,
            completion_matches: Vec::new(),
            completion_byte_range: None,
            completion_selected: 0,
            completion_prefix: String::new(),
            completion_block_trigger: false,
            show_outputs: false,
            show_outputs_requested: false,
            about_open: false,
            update_state: UpdateState::None,
            update_rx,
            update_tx,
            heading_numbered: true,
            last_auto_compile: None,
            last_content_change: None,
            tab_drag: None,
        };

        let mut args = std::env::args().skip(1);
        if let Some(file_path) = args.next() {
            let path = PathBuf::from(&file_path);
            if path.exists() {
                let tab = tab::Tab::load(&path);
                app.tabs.push(tab);
                app.active_tab = 0;
            }
        }

        app.apply_theme(cc.egui_ctx.clone());

        // Add Hack as fallback for Proportional to support Geometric Shapes (▲▼)
        // Ubuntu-Light and NotoEmoji lack U+25B2/U+25BC but Hack has full coverage.
        let mut fonts = egui::FontDefinitions::default();
        fonts
            .families
            .get_mut(&egui::FontFamily::Proportional)
            .unwrap()
            .push("Hack".to_owned());
        cc.egui_ctx.set_fonts(fonts);

        app
    }

    fn active_tab(&self) -> &tab::Tab {
        &self.tabs[self.active_tab]
    }

    fn active_tab_mut(&mut self) -> &mut tab::Tab {
        &mut self.tabs[self.active_tab]
    }

    fn apply_theme(&self, ctx: egui::Context) {
        let system_theme = ctx.system_theme();
        let is_dark = system_theme == Some(egui::Theme::Dark) || system_theme.is_none();
        let mut style = (*ctx.global_style()).clone();
        if is_dark {
            style.visuals = egui::Visuals::dark();
            style.visuals.selection.bg_fill = egui::Color32::from_rgb(180, 180, 180);
            style.visuals.panel_fill = egui::Color32::from_rgb(20, 20, 25);
            style.visuals.window_fill = egui::Color32::from_rgb(25, 25, 30);
            style.visuals.window_corner_radius = egui::CornerRadius::same(12);
            style.visuals.menu_corner_radius = egui::CornerRadius::same(8);
            style.visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(8);
            style.visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(8);
            style.visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(8);
            style.visuals.widgets.active.corner_radius = egui::CornerRadius::same(8);
            style.visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(35, 35, 40);
            style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(45, 45, 55);
            style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(65, 65, 80);
        } else {
            style.visuals = egui::Visuals::light();
        }
        ctx.set_global_style(style);
    }

    fn poll_events(&mut self) {
        for i in 0..self.tabs.len() {
            let tab = &mut self.tabs[i];
            while let Some(event) = tab.compiler.poll() {
                match event {
                    CompileEvent::Started => {
                        tab.status_message = "Compiling…".into();
                        tab.compile_start_time = Some(std::time::Instant::now());
                    }
                    CompileEvent::Warnings(warnings) => {
                        for w in &warnings {
                            tab.output_log.push((w.clone(), Color32::from_rgb(200, 180, 40)));
                        }
                    }
                    CompileEvent::Success(pdf_path) => {
                        let duration = tab
                            .compile_start_time
                            .take()
                            .map(|t| t.elapsed())
                            .unwrap_or_default();
                        tab.status_message = "Compilation succeeded".into();
                        tab.show_preview = true;
                        tab.error_lines.clear();
                        tab.error_message = None;
                        let dur_str = if duration.as_secs() > 0 {
                            format!("{}.{:02}s", duration.as_secs(), duration.subsec_millis() / 10)
                        } else {
                            format!("{}ms", duration.subsec_millis())
                        };
                        tab.output_log.clear();
                        tab.output_log.push((
                            format!("\u{221A} Compilation succeeded in {}", dur_str),
                            Color32::from_rgb(60, 180, 75),
                        ));
                        tab.preview.page = 0;
                        tab.preview.num_pages = None;
                        tab.preview.render_pdf(&pdf_path, 0);
                        self.show_outputs_requested = true;
                    }
                    CompileEvent::Failure(errors) => {
                        tab.compile_start_time.take();
                        tab.status_message = "Compilation failed".into();
                        tab.error_message = Some(errors.join("\n"));
                        tab.output_log.clear();
                        let re_line = error_line_regex();
                        let mut all_error_lines: Vec<usize> = Vec::new();
                        let mut j = 0;
                        while j < errors.len() {
                            let entry = &errors[j];
                            if entry.starts_with("! ") {
                                let err_code = entry[2..].trim().to_string();
                                let mut line_num = 0;
                                let mut context = String::new();
                                if j + 1 < errors.len() && errors[j + 1].starts_with("l.") {
                                    let line_ref = &errors[j + 1];
                                    if let Some(cap) = re_line.captures(line_ref) {
                                        if let Ok(n) = cap[1].parse() {
                                            line_num = n;
                                            all_error_lines.push(n);
                                        }
                                    }
                                    context = line_ref.find(' ')
                                        .map(|p| line_ref[p..].trim())
                                        .unwrap_or("")
                                        .to_string();
                                    j += 1;
                                }
                                let source_line = tab.buffer.text.lines().nth(
                                    line_num.checked_sub(1).unwrap_or(0),
                                ).unwrap_or("").to_string();
                                let error_syntax = if !source_line.is_empty() {
                                    source_line
                                } else if !context.is_empty() {
                                    context.clone()
                                } else {
                                    String::new()
                                };
                                let display = if !error_syntax.is_empty() {
                                    format!("\u{00D7} Error[line {}]: {} {}", line_num, error_syntax, err_code)
                                } else {
                                    format!("\u{00D7} Error: {}", err_code)
                                };
                                tab.output_log.push((display, Color32::from_rgb(220, 60, 60)));
                            }
                            j += 1;
                        }
                        all_error_lines.sort();
                        all_error_lines.dedup();
                        tab.error_lines = all_error_lines;
                        self.show_outputs_requested = true;
                    }
                }
            }

            while let Some(event) = tab.preview.poll() {
                match event {
                    PreviewEvent::NewImage(color_image) => {
                        tab.preview.current_image = Some(color_image);
                    }
                    PreviewEvent::Error(e) => {
                        if tab.preview.current_image.is_some() && tab.preview.page > 0 {
                            tab.preview.page -= 1;
                            tab.preview.num_pages = Some(tab.preview.page + 1);
                        } else {
                            tab.preview.render_error = Some(e);
                        }
                    }
                    PreviewEvent::Unsupported => {
                        tab.preview.render_error = Some(
                            "No PDF renderer found.\nInstall mupdf-tools (mutool), Ghostscript (gs), or poppler (pdftoppm)\nfor an embedded preview."
                                .into(),
                        );
                        tab.preview.open_externally();
                        tab.status_message = "PDF opened in external viewer".into();
                    }
                }
            }
        }
    }

    fn update_preview_textures(&mut self, ctx: &egui::Context) {
        for tab in &mut self.tabs {
            if let Some(img) = tab.preview.current_image.clone() {
                tab.preview_texture = Some(ctx.load_texture(
                    &format!("preview_{}", tab.title),
                    img,
                    egui::TextureOptions::LINEAR,
                ));
            }
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.apply_theme(ui.ctx().clone());

        self.process_update_messages();

        let is_fullscreen = ui.ctx().input(|i| i.viewport().fullscreen.unwrap_or(false));
        if is_fullscreen && ui.ctx().input(|i| i.key_pressed(egui::Key::Escape)) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
        }
        if ui.ctx().input(|i| i.key_pressed(egui::Key::F12)) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Fullscreen(!is_fullscreen));
        }

        if !self.tabs.is_empty() {
            if ui.ctx().input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Z)) {
                self.active_tab_mut().buffer.undo();
            }
            if ui.ctx().input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Y)) {
                self.active_tab_mut().buffer.redo();
            }
        }

        if let Some(action) = self.file_dialog_action.take() {
            match action {
                FileDialogAction::NewDocument => self.new_document(),
                FileDialogAction::Open => self.open_file(),
                FileDialogAction::Save => self.save_file(),
                FileDialogAction::SaveAs => self.save_as_file(),
            }
        }

        self.poll_events();
        if self.show_outputs_requested {
            self.show_outputs = true;
            self.show_outputs_requested = false;
        }
        self.update_preview_textures(ui.ctx());

        let now = Instant::now();
        if self.auto_compile && !self.tabs.is_empty() {
            let idle_time = self.last_content_change
                .map(|t| now.duration_since(t).as_secs_f32())
                .unwrap_or(f32::MAX);
            let compile_interval = self.last_auto_compile
                .map(|t| now.duration_since(t).as_secs_f32())
                .unwrap_or(f32::MAX);
            if idle_time >= 0.8 && compile_interval >= 1.2 {
                let tab = self.active_tab_mut();
                if let Some(p) = tab.buffer.path().map(|p| p.to_path_buf()) {
                    if tab.buffer.dirty {
                        let _ = std::fs::write(&p, &tab.buffer.text);
                        tab.compiler.compile(&p);
                    }
                }
                self.last_auto_compile = Some(now);
            }
        }

        if !is_fullscreen {
            Panel::top("menu_bar").show_inside(ui, |ui| {
                self.menu_bar(ui);
            });

            if !self.tabs.is_empty() {
                Panel::top("tab_bar")
                    .min_size(26.0)
                    .show_inside(ui, |ui| {
                        self.tab_bar(ui);
                    });
            }

            Panel::top("toolbar")
                .min_size(32.0)
                .show_inside(ui, |ui| {
                    self.toolbar(ui);
                });

            Panel::bottom("status_bar")
                .min_size(22.0)
                .show_inside(ui, |ui| {
                    self.status_bar(ui);
                });

            if !self.tabs.is_empty() {
                Panel::bottom("output_bar")
                    .min_size(24.0)
                    .show_inside(ui, |ui| {
                        self.output_bar(ui);
                    });
            }

            if self.show_outputs && !self.tabs.is_empty() {
                Panel::bottom("output_panel")
                    .resizable(true)
                    .default_size(150.0)
                    .min_size(60.0)
                    .show_inside(ui, |ui| {
                        self.output_panel(ui);
                    });
            }
        }

        CentralPanel::default().show_inside(ui, |ui| {
            if self.tabs.is_empty() {
                ui.vertical_centered(|ui| {
                    let avail = ui.available_height();
                    let content_height = 124.0;
                    let top = ((avail - content_height) / 2.0).max(0.0);
                    ui.add_space(top);
                    ui.heading("Lekhani Latex");
                    ui.add_space(8.0);
                    ui.colored_label(
                        Color32::GRAY,
                        "Create a new document or open an existing one to get started.",
                    );
                    ui.add_space(12.0);
                    if ui.button("  New Document  ").clicked() {
                        self.new_document();
                    }
                    ui.add_space(4.0);
                    if ui.button("  Open File  ").clicked() {
                        self.open_file();
                    }
                });
                return;
            }

            ui.vertical_centered(|ui| {
                ui.horizontal(|ui| {
                    self.formatting_toolbar(ui);
                    if self.active_tab().show_preview {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            self.preview_toolbar(ui);
                        });
                    }
                });
            });
            ui.add_space(4.0);

            if self.active_tab().show_preview {
                let half = (ui.available_width() / 2.0).clamp(200.0, 800.0);
                Panel::right("preview_panel")
                    .resizable(true)
                    .default_size(half)
                    .min_size(200.0)
                    .show_inside(ui, |ui| {
                        self.preview_content(ui);
                    });
            }
            self.editor_area(ui);
        });

        if is_fullscreen {
            egui::Area::new(egui::Id::new("fullscreen_toast"))
                .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -24.0])
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    egui::Frame::NONE
                        .fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180))
                        .corner_radius(egui::CornerRadius::same(8))
                        .inner_margin(egui::Margin::symmetric(16, 8))
                        .shadow(egui::epaint::Shadow {
                            offset: [0, 4],
                            blur: 8,
                            spread: 0,
                            color: egui::Color32::from_black_alpha(80),
                        })
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new("Press Esc to exit fullscreen")
                                    .color(egui::Color32::WHITE)
                                    .size(13.0),
                            );
                        });
                });
        }

        #[allow(deprecated)]
        if self.about_open {
            let mut about_open = self.about_open;
            ui.ctx().show_viewport_immediate(
                egui::ViewportId::from_hash_of("about_viewport"),
                egui::ViewportBuilder::default()
                    .with_title("About")
                    .with_inner_size([700.0, 380.0])
                    .with_resizable(false)
                    .with_maximize_button(false)
                    .with_minimize_button(false),
                |ctx, _class| {
                    if ctx.input(|i| i.viewport().close_requested()) {
                        about_open = false;
                    }

                    let is_light = ctx.system_theme() == Some(egui::Theme::Light);
                    let mut style = (*ctx.global_style()).clone();
                    if is_light {
                        style.visuals = egui::Visuals::light();
                    }

                    let bg_fill = if is_light { egui::Color32::from_rgb(245, 245, 245) } else { ctx.global_style().visuals.window_fill };

                    egui::Panel::bottom("about_bottom_panel")
                        .frame(egui::Frame::default().inner_margin(egui::Margin { left: 16, right: 16, top: 8, bottom: 16 }).fill(bg_fill))
                        .show(ctx, |ui| {
                            ui.set_style(style.clone());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button(egui::RichText::new("Close").size(14.0)).clicked() {
                                    about_open = false;
                                }
                            });
                        });

                    egui::CentralPanel::default()
                        .frame(egui::Frame::default().inner_margin(16).fill(bg_fill))
                        .show(ctx, |ui| {
                            ui.set_style(style);
                            ui.vertical(|ui| {
                                ui.heading(egui::RichText::new("Lekhani Latex").size(24.0).strong());

                                ui.add_space(12.0);
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(egui::RichText::new("Lekhani Latex is a modern, easy-to-use, open source LaTeX editor with live preview, syntax highlighting, and autocompletion.").size(13.0));
                                });

                                ui.add_space(10.0);
                                ui.label(egui::RichText::new("Copyright © 2026-Present Borneel B. Phukan.").size(12.0));

                                ui.add_space(10.0);
                                ui.horizontal(|ui| {
                                    ui.add_space(40.0);
                                    ui.hyperlink_to(egui::RichText::new("Credits").size(13.0), "https://github.com/example/lekhani-latex/credits");
                                    ui.add_space(8.0);
                                    ui.hyperlink_to(egui::RichText::new("Website").size(13.0), "https://github.com/example/lekhani-latex");
                                    ui.add_space(8.0);
                                    ui.hyperlink_to(egui::RichText::new("Release Notes").size(13.0), "https://github.com/example/lekhani-latex/releases");
                                });

                                ui.add_space(20.0);
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("Version Information").size(14.0).strong());
                                    if ui.button("\u{1F4CB}").on_hover_text("Copy Version Info").clicked() {
                                        let version_info = format!(
                                            "Version: {} (Stable)\nEnvironment: OS: {} ({}); Arch: {}",
                                            env!("CARGO_PKG_VERSION"), std::env::consts::OS, std::env::consts::FAMILY, std::env::consts::ARCH
                                        );
                                        ctx.copy_text(version_info);
                                    }
                                });

                                ui.add_space(4.0);
                                egui::Grid::new("about_version_grid").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                                    ui.label(egui::RichText::new("Version:").size(12.0));
                                    ui.label(egui::RichText::new(format!("{} (Stable)", env!("CARGO_PKG_VERSION"))).size(12.0));
                                    ui.end_row();

                                    ui.label(egui::RichText::new("Environment:").size(12.0));
                                    ui.label(egui::RichText::new(format!("OS: {} ({}); Arch: {}", std::env::consts::OS, std::env::consts::FAMILY, std::env::consts::ARCH)).size(12.0));
                                    ui.end_row();
                                });
                            });
                        });
                }
            );
            self.about_open = about_open;
        }

        self.ui_update_dialog(ui);
    }

    fn on_exit(&mut self) {
        for tab in &mut self.tabs {
            // Release GPU textures before the rendering context is destroyed
            tab.preview_texture = None;
            if tab.buffer.dirty {
                if let Some(path) = tab.buffer.path().map(|p| p.to_path_buf()) {
                    let _ = tab.buffer.save_as(&path);
                }
            }
        }
    }
}

fn tab_width_for(tabs: &[tab::Tab], i: usize, is_active: bool) -> f32 {
    let title_w = (tabs[i].title.len() as f32) * 7.5 + 16.0;
    let close_w = if is_active { 28.0 } else { 0.0 };
    (title_w + close_w).max(40.0)
}

impl App {
    fn check_for_updates(&mut self, ctx: egui::Context) {
        self.update_state = UpdateState::Checking;
        let tx = self.update_tx.clone();
        std::thread::spawn(move || {
            let url = "https://api.github.com/repos/example/lekhani-latex/releases/latest";
            let mut is_available = false;
            let mut version = None;
            if let Ok(response) =
                ureq::get(url).header("User-Agent", "lekhani-latex").call()
            {
                use std::io::Read;
                let mut json = String::new();
                if response
                    .into_body()
                    .into_reader()
                    .read_to_string(&mut json)
                    .is_ok()
                {
                    if let Some(tag_idx) = json.find("\"tag_name\":") {
                        let rest = &json[tag_idx + 11..];
                        if let Some(start_quote) = rest.find('"') {
                            let rest = &rest[start_quote + 1..];
                            if let Some(end_quote) = rest.find('"') {
                                let tag = &rest[..end_quote];
                                let latest_version = tag.trim_start_matches('v');
                                if latest_version != env!("CARGO_PKG_VERSION") {
                                    is_available = true;
                                    version = Some(latest_version.to_string());
                                }
                            }
                        }
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
            let _ = tx.send(UpdateMessage::CheckResult(is_available, version));
            ctx.request_repaint();
        });
    }

    fn download_update(&mut self, version: String, ctx: egui::Context) {
        self.update_state = UpdateState::Downloading(0.0);
        let tx = self.update_tx.clone();
        std::thread::spawn(move || {
            let url = format!("https://github.com/example/lekhani-latex/releases/download/v{}/lekhani-latexSetup.exe", version);
            match ureq::get(&url).header("User-Agent", "lekhani-latex").call() {
                Ok(response) => {
                    let len: Option<u64> = response
                        .headers()
                        .get("Content-Length")
                        .and_then(|h| h.to_str().ok())
                        .and_then(|s| s.parse().ok());
                    let mut reader = response.into_body().into_reader();
                    let download_dir =
                        dirs::download_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
                    let out_path = download_dir.join("lekhani-latexSetup.exe");
                    if let Ok(mut file) = std::fs::File::create(&out_path) {
                        use std::io::Read;
                        let mut buf = [0; 8192];
                        let mut downloaded = 0;
                        loop {
                            match reader.read(&mut buf) {
                                Ok(0) => break,
                                Ok(n) => {
                                    use std::io::Write;
                                    let _ = file.write_all(&buf[..n]);
                                    downloaded += n as u64;
                                    if let Some(total) = len {
                                        let progress =
                                            (downloaded as f32) / (total as f32);
                                        let _ = tx.send(UpdateMessage::DownloadProgress(progress));
                                        ctx.request_repaint();
                                    }
                                }
                                Err(_) => {
                                    let _ = tx.send(UpdateMessage::DownloadComplete(
                                        Err("Download failed".into()),
                                    ));
                                    ctx.request_repaint();
                                    return;
                                }
                            }
                        }
                        let _ = tx.send(UpdateMessage::DownloadComplete(
                            Ok(out_path.to_string_lossy().into()),
                        ));
                    } else {
                        let _ = tx.send(UpdateMessage::DownloadComplete(
                            Err("Failed to create file".into()),
                        ));
                    }
                    ctx.request_repaint();
                }
                Err(_) => {
                    let _ = tx.send(UpdateMessage::DownloadComplete(Err(
                        "Download failed".into(),
                    )));
                    ctx.request_repaint();
                }
            }
        });
    }

    fn process_update_messages(&mut self) {
        while let Ok(msg) = self.update_rx.try_recv() {
            match msg {
                UpdateMessage::CheckResult(is_available, version) => {
                    if is_available {
                        self.update_state =
                            UpdateState::Prompt(version.unwrap_or_else(|| "unknown".into()));
                    } else {
                        self.update_state = UpdateState::None;
                        rfd::MessageDialog::new()
                            .set_title("No Update")
                            .set_description("No update available.")
                            .set_level(rfd::MessageLevel::Warning)
                            .show();
                    }
                }
                UpdateMessage::DownloadProgress(progress) => {
                    self.update_state = UpdateState::Downloading(progress);
                }
                UpdateMessage::DownloadComplete(result) => {
                    self.update_state = UpdateState::None;
                    match result {
                        Ok(path) => {
                            if let Err(_) = std::process::Command::new(&path).spawn() {
                                // Toast notification not implemented; update state already None
                            } else {
                                std::process::exit(0);
                            }
                        }
                        Err(_) => {}
                    }
                }
            }
        }
    }

    fn ui_update_dialog(&mut self, ui: &mut egui::Ui) {
        match self.update_state.clone() {
            UpdateState::None => {}
            UpdateState::Checking => {
                let mut is_open = true;
                egui::Window::new("Checking for Updates")
                    .collapsible(false)
                    .resizable(false)
                    .open(&mut is_open)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .frame(
                        egui::Frame::window(&ui.ctx().global_style())
                            .inner_margin(16.0)
                            .corner_radius(8),
                    )
                    .show(ui.ctx(), |ui| {
                        ui.vertical_centered(|ui| {
                            ui.label("Checking for newer version...");
                            ui.add_space(8.0);
                            ui.spinner();
                        });
                    });
                if !is_open {
                    self.update_state = UpdateState::None;
                }
            }
            UpdateState::Prompt(version) => {
                let is_dark =
                    ui.ctx().system_theme().unwrap_or(egui::Theme::Dark) == egui::Theme::Dark;

                let bg_color = if is_dark {
                    egui::Color32::from_rgb(45, 45, 55)
                } else {
                    egui::Color32::from_rgb(240, 240, 245)
                };
                let text_color = if is_dark {
                    egui::Color32::from_rgb(240, 240, 245)
                } else {
                    egui::Color32::from_rgb(20, 20, 25)
                };

                let btn_inactive = if is_dark {
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 20)
                } else {
                    egui::Color32::from_rgba_unmultiplied(0, 0, 0, 15)
                };
                let btn_hovered = if is_dark {
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 40)
                } else {
                    egui::Color32::from_rgba_unmultiplied(0, 0, 0, 30)
                };
                let btn_active = if is_dark {
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 60)
                } else {
                    egui::Color32::from_rgba_unmultiplied(0, 0, 0, 45)
                };

                egui::Area::new(egui::Id::new("update_banner"))
                    .anchor(egui::Align2::CENTER_TOP, [0.0, 10.0])
                    .order(egui::Order::Foreground)
                    .show(ui.ctx(), |ui| {
                        egui::Frame::window(&ui.ctx().global_style())
                            .fill(bg_color)
                            .corner_radius(egui::CornerRadius::same(6))
                            .inner_margin(egui::Margin::symmetric(16, 10))
                            .shadow(egui::epaint::Shadow {
                                offset: [0, 4],
                                blur: 16,
                                spread: 0,
                                color: egui::Color32::from_black_alpha(80),
                            })
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "New update available: v{}",
                                            version
                                        ))
                                        .color(text_color)
                                        .strong(),
                                    );
                                    ui.add_space(20.0);

                                    ui.visuals_mut().widgets.inactive.bg_fill = btn_inactive;
                                    ui.visuals_mut().widgets.hovered.bg_fill = btn_hovered;
                                    ui.visuals_mut().widgets.active.bg_fill = btn_active;

                                    if ui
                                        .button(
                                            egui::RichText::new("Download Now").color(text_color),
                                        )
                                        .clicked()
                                    {
                                        self.update_state = UpdateState::Downloading(0.0);
                                        self.download_update(version.clone(), ui.ctx().clone());
                                    }
                                    if ui
                                        .button(egui::RichText::new("Skip").color(text_color))
                                        .clicked()
                                    {
                                        self.update_state = UpdateState::None;
                                    }
                                });
                            });
                    });
            }
            UpdateState::Downloading(progress) => {
                let mut is_open = true;
                egui::Window::new("Downloading Update")
                    .collapsible(false)
                    .resizable(false)
                    .open(&mut is_open)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .frame(
                        egui::Frame::window(&ui.ctx().global_style())
                            .inner_margin(16.0)
                            .corner_radius(8),
                    )
                    .show(ui.ctx(), |ui| {
                        ui.vertical_centered(|ui| {
                            ui.label(egui::RichText::new("Downloading update...").size(14.0));
                            ui.add_space(12.0);
                            let rect = ui.available_rect_before_wrap();
                            let size = egui::vec2(rect.width(), 20.0);
                            let (_rect, _response) =
                                ui.allocate_exact_size(size, egui::Sense::hover());
                            let corner_radius = egui::CornerRadius::same(4);
                            ui.painter().rect_filled(
                                _rect,
                                corner_radius,
                                ui.visuals().extreme_bg_color,
                            );
                            let fill_width = _rect.width() * progress;
                            if fill_width > 0.0 {
                                let fill_rect = egui::Rect::from_min_size(
                                    _rect.min,
                                    egui::vec2(fill_width, _rect.height()),
                                );
                                ui.painter().rect_filled(
                                    fill_rect,
                                    corner_radius,
                                    ui.visuals().selection.bg_fill,
                                );
                            }
                            ui.painter().text(
                                _rect.center(),
                                egui::Align2::CENTER_CENTER,
                                format!("{:.0}%", progress * 100.0),
                                egui::FontId::proportional(14.0),
                                ui.visuals().text_color(),
                            );
                            ui.add_space(16.0);
                            if ui.button("Cancel").clicked() {
                                self.update_state = UpdateState::None;
                            }
                        });
                    });
                if !is_open {
                    self.update_state = UpdateState::None;
                }
            }
        }
    }

    fn tab_bar(&mut self, ui: &mut egui::Ui) {
        let mut remove_tab = None;
        let tab_count = self.tabs.len();
        let pointer_down = ui.ctx().input(|i| i.pointer.button_down(egui::PointerButton::Primary));
        let pointer_up = ui.ctx().input(|i| i.pointer.button_released(egui::PointerButton::Primary));
        let pointer_pos = ui.ctx().input(|i| i.pointer.interact_pos());
        let press_origin = ui.ctx().input(|i| i.pointer.press_origin());

        if pointer_up || !pointer_down {
            if let Some((from_idx, _)) = self.tab_drag.take() {
                if pointer_up {
                    let mut to = tab_count.saturating_sub(1);
                    if let Some(pos) = pointer_pos {
                        let mut cx = ui.min_rect().left();
                        for j in 0..tab_count {
                            let w = tab_width_for(&self.tabs, j, j == self.active_tab);
                            let center = cx + w / 2.0;
                            if pos.x < center { to = j; break; }
                            cx += w;
                        }
                    }
                    let from = from_idx;
                    if from != to && from < self.tabs.len() {
                        let was_active = self.active_tab == from;
                        let tab = self.tabs.remove(from);
                        let insert_at = if to > from { to - 1 } else { to };
                        self.tabs.insert(insert_at, tab);
                        if was_active {
                            self.active_tab = insert_at;
                        } else if from < self.active_tab && insert_at >= self.active_tab {
                            self.active_tab -= 1;
                        } else if from > self.active_tab && insert_at <= self.active_tab {
                            self.active_tab += 1;
                        }
                    }
                }
            }
        }

        let mut drag_rects: Vec<egui::Rect> = Vec::new();

        ui.scope(|ui| {
            let is_dark = ui.visuals().dark_mode;

            if is_dark {
                ui.style_mut().visuals.selection.bg_fill = Color32::from_rgb(38, 38, 38);
                ui.style_mut().visuals.widgets.hovered.bg_fill = Color32::from_rgb(45, 45, 45);
            } else {
                ui.style_mut().visuals.selection.bg_fill =
                    ui.style().visuals.widgets.inactive.weak_bg_fill;
                ui.style_mut().visuals.widgets.hovered.bg_fill =
                    ui.style().visuals.widgets.hovered.weak_bg_fill;
            }

            ui.style_mut().visuals.widgets.active.corner_radius = egui::CornerRadius::same(4);
            ui.style_mut().visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(4);
            ui.style_mut().visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(4);
            ui.style_mut().visuals.widgets.noninteractive.corner_radius =
                egui::CornerRadius::same(4);

            ui.style_mut()
                .text_styles
                .insert(egui::TextStyle::Button, egui::FontId::proportional(13.0));
            ui.style_mut()
                .text_styles
                .insert(egui::TextStyle::Body, egui::FontId::proportional(13.0));
            ui.style_mut().spacing.button_padding = egui::vec2(10.0, 6.0);

            if !self.tabs.is_empty() {
                ui.horizontal(|ui| {
                    for i in 0..tab_count {
                        let is_dragging = self.tab_drag.map_or(false, |(idx, _)| idx == i);
                        let is_active = i == self.active_tab;
                        let title = if self.tabs[i].buffer.dirty {
                            format!("{} •", self.tabs[i].title)
                        } else {
                            self.tabs[i].title.clone()
                        };

                        if is_dragging {
                            let tw = tab_width_for(&self.tabs, i, is_active);
                            let (rect, _) = ui.allocate_exact_size(
                                egui::vec2(tw, 28.0),
                                egui::Sense::hover(),
                            );
                            drag_rects.push(rect);
                            continue;
                        }

                        let text = format!("{}      ", title);
                        let text_color = if is_active && is_dark {
                            Color32::from_rgb(220, 220, 220)
                        } else {
                            ui.visuals().text_color()
                        };
                        let text_style = if is_active {
                            egui::RichText::new(&text)
                                .color(text_color)
                                .strong()
                                .size(13.0)
                        } else {
                            egui::RichText::new(&text).color(text_color).size(13.0)
                        };

                        let resp = ui.selectable_label(is_active, text_style);
                        if resp.clicked() {
                            self.active_tab = i;
                        }
                        drag_rects.push(resp.rect);

                        if is_active {
                            let close_rect = egui::Rect::from_min_size(
                                egui::pos2(
                                    resp.rect.right() - 22.0,
                                    resp.rect.top() + (resp.rect.height() - 16.0) / 2.0 - 2.0,
                                ),
                                egui::vec2(16.0, 16.0),
                            );
                            if ui
                                .put(
                                    close_rect,
                                    egui::Button::new(egui::RichText::new("×").size(13.0))
                                        .frame(false),
                                )
                                .clicked()
                            {
                                if !self.tabs[i].buffer.dirty
                                    || rfd::MessageDialog::new()
                                        .set_title("Unsaved Changes")
                                        .set_description(
                                            "You have unsaved changes. Close this workspace?",
                                        )
                                        .set_buttons(rfd::MessageButtons::YesNo)
                                        .show()
                                        == rfd::MessageDialogResult::Yes
                                {
                                    remove_tab = Some(i);
                                }
                            }
                        }
                    }
                });
            }
        });

        if self.tab_drag.is_none() && pointer_down && !pointer_up {
            if let (Some(origin), Some(pos)) = (press_origin, pointer_pos) {
                if (pos.x - origin.x).abs() > 6.0 {
                    for (i, rect) in drag_rects.iter().enumerate() {
                        if rect.contains(origin) {
                            self.tab_drag = Some((i, rect.left()));
                            break;
                        }
                    }
                }
            }
        }

        if let Some((drag_idx, _)) = self.tab_drag {
            if let Some(pos) = pointer_pos {
                let is_dark = ui.visuals().dark_mode;
                let active_fill = if is_dark { Color32::from_rgb(38, 38, 38) } else { Color32::from_rgb(225, 226, 232) };
                let w = tab_width_for(&self.tabs, drag_idx, drag_idx == self.active_tab);
                let overlay_y = drag_rects.first().map_or(0.0, |r| r.top());

                let overlay_pos = egui::pos2(pos.x - w / 2.0, overlay_y);
                egui::Area::new(egui::Id::new("tab_drag_overlay"))
                    .fixed_pos(overlay_pos)
                    .order(egui::Order::Foreground)
                    .show(ui.ctx(), |ui| {
                        let title = if self.tabs[drag_idx].buffer.dirty {
                            format!("{} •", self.tabs[drag_idx].title)
                        } else {
                            self.tabs[drag_idx].title.clone()
                        };
                        egui::Frame::NONE
                            .fill(active_fill)
                            .inner_margin(egui::Margin::symmetric(6, 2))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.set_min_size(egui::vec2(40.0, 26.0));
                                    ui.add(egui::Label::new(&title));
                                    if drag_idx == self.active_tab {
                                        ui.add_sized(egui::vec2(18.0, 18.0), egui::Label::new("×"));
                                    }
                                });
                            });
                    });
            }
        }

        if let Some(idx) = remove_tab {
            self.tabs.remove(idx);
            if !self.tabs.is_empty() {
                self.active_tab = self.active_tab.min(self.tabs.len() - 1);
            }
        }
    }

    fn output_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let is_open = self.show_outputs;
            let label = if is_open { "▼ Outputs" } else { "▶ Outputs" };
            if ui.button(label).clicked() {
                self.show_outputs = !self.show_outputs;
            }
        });
    }

    fn output_panel(&mut self, ui: &mut egui::Ui) {
        let log = self.active_tab().output_log.clone();
        let text_color = ui.style().visuals.text_color();
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new("Output").strong(),
            );
            ui.separator();
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    if log.is_empty() {
                        ui.colored_label(
                            egui::Color32::GRAY,
                            "No output to display.",
                        );
                    } else {
                        for (text, color) in &log {
                            let c = if *color == egui::Color32::WHITE {
                                text_color
                            } else {
                                *color
                            };
                            ui.colored_label(c, text);
                        }
                    }
                });
        });
    }
}
