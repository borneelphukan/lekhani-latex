pub mod tab;
mod toolbar;
mod menubar;
mod statusbar;
mod preview_panel;
mod editor;

use std::path::PathBuf;
use std::time::Instant;

use egui::{CentralPanel, Color32, Panel};
use regex::Regex;

use crate::compiler::CompileEvent;
use crate::preview::PreviewEvent;
use crate::types::*;

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
    system_dark: bool,
    last_auto_compile: Option<Instant>,
}

enum FileDialogAction {
    Open,
    Save,
    SaveAs,
    NewDocument,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
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
            system_dark: cc.egui_ctx.global_style().visuals.dark_mode,
            last_auto_compile: None,
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
        let resolved = match self.theme {
            Theme::System => {
                if self.system_dark { Theme::Dark } else { Theme::Light }
            }
            theme => theme,
        };
        let mut style = (*ctx.global_style()).clone();
        match resolved {
            Theme::Dark => {
                style.visuals = egui::Visuals::dark();
            }
            Theme::Light => {
                style.visuals = egui::Visuals::light();
            }
            Theme::System => unreachable!(),
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
                        let re_line = Regex::new(r"l\.(\d+)").unwrap();
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
            if self.last_auto_compile.map_or(true, |t| now.duration_since(t).as_secs_f32() >= 1.5) {
                let tab = self.active_tab_mut();
                if tab.buffer.path().is_some() && tab.buffer.dirty {
                    if tab.buffer.save().is_ok() {
                        if let Some(p) = tab.buffer.path().map(|p| p.to_path_buf()) {
                            tab.compiler.compile(&p);
                        }
                    }
                }
                self.last_auto_compile = Some(now);
            }
        }

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

        CentralPanel::default().show_inside(ui, |ui| {
            if self.tabs.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(120.0);
                    ui.heading("LaTeX Writer");
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

            if self.active_tab().show_preview {
                let half = (ui.available_width() / 2.0).clamp(200.0, 800.0);
                Panel::right("preview_panel")
                    .resizable(true)
                    .default_size(half)
                    .min_size(200.0)
                    .show_inside(ui, |ui| {
                        self.preview_panel(ui);
                    });
            }
            self.editor_area(ui);
        });

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

impl App {
    fn tab_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let mut remove_tab = None;
            for i in 0..self.tabs.len() {
                let is_active = i == self.active_tab;
                let title = if self.tabs[i].buffer.dirty {
                    format!("{} •", self.tabs[i].title)
                } else {
                    self.tabs[i].title.clone()
                };

                egui::Frame::NONE
                    .inner_margin(egui::Margin::symmetric(6, 2))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.set_min_size(egui::vec2(40.0, 20.0));
                            let resp = ui.add(egui::Button::new(&title));
                            if resp.clicked() {
                                self.active_tab = i;
                            }
                            if is_active {
                                let close_resp = ui.add_sized(
                                    egui::vec2(14.0, 14.0),
                                    egui::Label::new("×")
                                        .sense(egui::Sense::click()),
                                );
                                if close_resp.clicked() {
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
                        });
                    });
            }
            if let Some(idx) = remove_tab {
                self.tabs.remove(idx);
                if !self.tabs.is_empty() {
                    self.active_tab = self.active_tab.min(self.tabs.len() - 1);
                }
            }
        });
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
