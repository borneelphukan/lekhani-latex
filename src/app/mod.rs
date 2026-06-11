pub mod tab;
mod toolbar;
mod menubar;
mod statusbar;
mod preview_panel;
mod editor;

use std::path::PathBuf;

use egui::{CentralPanel, Color32, Panel, ScrollArea};

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
        app
    }

    fn active_tab(&self) -> &tab::Tab {
        &self.tabs[self.active_tab]
    }

    fn active_tab_mut(&mut self) -> &mut tab::Tab {
        &mut self.tabs[self.active_tab]
    }

    fn apply_theme(&self, ctx: egui::Context) {
        let mut style = (*ctx.global_style()).clone();
        let resolved = match self.theme {
            Theme::System => {
                if style.visuals.dark_mode {
                    Theme::Dark
                } else {
                    Theme::Light
                }
            }
            theme => theme,
        };
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
        for tab in &mut self.tabs {
            while let Some(event) = tab.compiler.poll() {
                match event {
                    CompileEvent::Started => {
                        tab.status_message = "Compiling…".into();
                    }
                    CompileEvent::Success(pdf_path) => {
                        tab.status_message = "Compilation succeeded".into();
                        tab.show_preview = true;
                        tab.preview.page = 0;
                        tab.preview.num_pages = None;
                        tab.preview.render_pdf(&pdf_path, 0);
                    }
                    CompileEvent::Failure(errors) => {
                        tab.status_message = "Compilation failed".into();
                        tab.error_message = Some(errors.join("\n"));
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
        self.update_preview_textures(ui.ctx());

        if !self.tabs.is_empty() {
            Panel::top("tab_bar")
                .min_size(26.0)
                .show_inside(ui, |ui| {
                    self.tab_bar(ui);
                });
        }

        Panel::top("menu_bar").show_inside(ui, |ui| {
            self.menu_bar(ui);
        });

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

        if !self.tabs.is_empty() {
            let err = self.active_tab_mut().error_message.take();
            if let Some(msg) = err {
                let mut keep = true;
                egui::Window::new("Error")
                    .collapsible(false)
                    .resizable(true)
                    .show(ui.ctx(), |ui| {
                        ScrollArea::vertical()
                            .max_height(300.0)
                            .show(ui, |ui| {
                                ui.label(&msg);
                            });
                        if ui.button("Close").clicked() {
                            keep = false;
                        }
                    });
                if keep {
                    self.active_tab_mut().error_message = Some(msg);
                }
            }
        }
    }

    fn on_exit(&mut self) {
        for tab in &mut self.tabs {
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

                let bg = if is_active {
                    self.theme.active_tab_bg(ui.ctx())
                } else {
                    Color32::TRANSPARENT
                };

                egui::Frame::NONE
                    .fill(bg)
                    .inner_margin(egui::Margin::symmetric(6, 2))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.set_min_size(egui::vec2(40.0, 20.0));
                            let resp =
                                ui.add(egui::Button::new(&title).selected(is_active));
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
                                    remove_tab = Some(i);
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
}
