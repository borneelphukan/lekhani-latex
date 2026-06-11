use std::path::{Path, PathBuf};

use egui::widgets::Image;
use egui::{CentralPanel, Color32, FontId, Panel, ScrollArea};
use egui::TextEdit;

use crate::buffer::EditorBuffer;
use crate::completions;
use crate::compiler::{CompileEvent, CompilerBridge, CompileStatus};
use crate::lexer;
use crate::preview::{PreviewEvent, PreviewViewer};
use crate::types::*;

pub struct Tab {
    pub title: String,
    pub buffer: EditorBuffer,
    pub compiler: CompilerBridge,
    pub preview: PreviewViewer,
    pub show_preview: bool,
    pub preview_texture: Option<egui::TextureHandle>,
    pub status_message: String,
    pub error_message: Option<String>,
    pub scroll_offset: egui::Vec2,
}

impl Tab {
    fn load(path: &Path) -> Self {
        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled")
            .to_string();
        let buffer = EditorBuffer::load(path).unwrap_or_else(|_| {
            let mut b = EditorBuffer::new();
            b.set_path(Some(path.to_path_buf()));
            b
        });
        Self {
            title,
            buffer,
            compiler: CompilerBridge::new(CompilerConfig::default()),
            preview: PreviewViewer::new(),
            show_preview: true,
            preview_texture: None,
            status_message: format!("Opened {}", path.display()),
            error_message: None,
            scroll_offset: egui::Vec2::ZERO,
        }
    }

    fn new_empty(path: &Path) -> Self {
        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled")
            .to_string();
        let mut buffer = EditorBuffer::new();
        buffer.set_path(Some(path.to_path_buf()));
        buffer.dirty = false;
        Self {
            title,
            buffer,
            compiler: CompilerBridge::new(CompilerConfig::default()),
            preview: PreviewViewer::new(),
            show_preview: true,
            preview_texture: None,
            status_message: format!("Created {}", path.display()),
            error_message: None,
            scroll_offset: egui::Vec2::ZERO,
        }
    }
}

pub struct App {
    tabs: Vec<Tab>,
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
        let initial_theme = if cc.egui_ctx.global_style().visuals.dark_mode {
            Theme::Dark
        } else {
            Theme::Light
        };

        let mut app = Self {
            tabs: Vec::new(),
            active_tab: 0,
            theme: initial_theme,
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
                let tab = Tab::load(&path);
                app.tabs.push(tab);
                app.active_tab = 0;
            }
        }

        app.apply_theme(cc.egui_ctx.clone());
        app
    }

    fn active_tab(&self) -> &Tab {
        &self.tabs[self.active_tab]
    }

    fn active_tab_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active_tab]
    }

    fn apply_theme(&self, ctx: egui::Context) {
        let mut style = (*ctx.global_style()).clone();
        match self.theme {
            Theme::Dark => {
                style.visuals = egui::Visuals::dark();
            }
            Theme::Light => {
                style.visuals = egui::Visuals::light();
            }
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
                            // Page doesn't exist – revert to previous and store page count
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

        // Tab bar
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

        // Error window for active tab
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

// --- Tab bar ---
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
                    if self.theme == Theme::Dark {
                        Color32::from_rgb(40, 44, 52)
                    } else {
                        Color32::from_rgb(240, 240, 244)
                    }
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

// --- Toolbar ---
impl App {
    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let compile_enabled = !self.tabs.is_empty()
                && self.active_tab().buffer.path().is_some();
            let compile_btn = egui::Button::new("  \u{25B6}  Compile  ")
                .min_size(egui::vec2(90.0, 26.0))
                .fill(if self.theme == Theme::Dark {
                    egui::Color32::from_rgb(30, 120, 60)
                } else {
                    egui::Color32::from_rgb(40, 160, 80)
                });
            let resp = ui.add_enabled(compile_enabled, compile_btn);
            if resp.clicked() {
                self.trigger_compile();
            }

            ui.separator();

            ui.checkbox(&mut self.auto_compile, "Auto-compile");
        });
    }
}

// --- Menu bar ---
impl App {
    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New Document").clicked() {
                    ui.close();
                    self.file_dialog_action =
                        Some(FileDialogAction::NewDocument);
                }
                if ui.button("Open…").clicked() {
                    ui.close();
                    self.file_dialog_action =
                        Some(FileDialogAction::Open);
                }
                let has_tabs = !self.tabs.is_empty();
                if ui.add_enabled(has_tabs, egui::Button::new("Save")).clicked() {
                    ui.close();
                    self.file_dialog_action =
                        Some(FileDialogAction::Save);
                }
                if ui.add_enabled(has_tabs, egui::Button::new("Save As…")).clicked() {
                    ui.close();
                    self.file_dialog_action =
                        Some(FileDialogAction::SaveAs);
                }
                ui.separator();
                if ui.button("Quit").clicked() {
                    ui.close();
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });

            let has_tabs = !self.tabs.is_empty();
            if has_tabs {
                ui.menu_button("Edit", |ui| {
                    if ui.button("Undo").clicked() {
                        ui.close();
                        self.active_tab_mut().buffer.undo();
                    }
                    if ui.button("Redo").clicked() {
                        ui.close();
                        self.active_tab_mut().buffer.redo();
                    }
                });
            } else {
                ui.menu_button("Edit", |_ui| {});
            }

            if has_tabs {
                ui.menu_button("Build", |ui| {
                    ui.checkbox(
                        &mut self.auto_compile,
                        "Auto-compile on save",
                    );
                });
            } else {
                ui.menu_button("Build", |_ui| {});
            }

            ui.menu_button("View", |ui| {
                if ui.add_enabled(has_tabs, egui::Button::new("Toggle Preview")).clicked() {
                    ui.close();
                    let tab = self.active_tab_mut();
                    tab.show_preview = !tab.show_preview;
                    tab.status_message = if tab.show_preview {
                        "Preview shown".into()
                    } else {
                        "Preview hidden".into()
                    };
                }
                if ui.button("Toggle Theme").clicked() {
                    ui.close();
                    self.theme = match self.theme {
                        Theme::Dark => Theme::Light,
                        Theme::Light => Theme::Dark,
                    };
                    self.apply_theme(ui.ctx().clone());
                    ui.ctx().request_repaint();
                }
            });
        });
    }

    fn trigger_compile(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        let tab = self.active_tab_mut();
        let path = tab.buffer.path().map(|p| p.to_path_buf());
        if let Some(ref path) = path {
            if let Err(e) = tab.buffer.save() {
                tab.error_message =
                    Some(format!("Failed to save before compile: {}", e));
                return;
            }
            tab.compiler.compile(path);
        } else {
            tab.error_message =
                Some("Please save the file before compiling.".into());
        }
    }

    fn new_document(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("LaTeX", &["tex"])
            .set_file_name("document.tex")
            .save_file()
        {
            let parent = path.parent().unwrap_or(Path::new("."));
            let file_name =
                path.file_name().unwrap_or(std::ffi::OsStr::new("document.tex"));
            let project_folder = parent.join(
                path.file_stem()
                    .unwrap_or(std::ffi::OsStr::new("document")),
            );
            let tex_path = project_folder.join(file_name);

            // Write template and open in a tab
            let mut buf = EditorBuffer::new();
            if let Err(e) = buf.save_as(&tex_path) {
                self.active_tab_mut().error_message =
                    Some(format!("Failed to create document: {}", e));
                return;
            }

            let tab = Tab::new_empty(&tex_path);
            self.tabs.push(tab);
            self.active_tab = self.tabs.len() - 1;
        }
    }

    fn open_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("LaTeX", &["tex", "sty", "cls"])
            .pick_file()
        {
            let tab = Tab::load(&path);
            self.tabs.push(tab);
            self.active_tab = self.tabs.len() - 1;
        }
    }

    fn save_file(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        let auto = self.auto_compile;
        let tab = self.active_tab_mut();
        if tab.buffer.path().is_some() {
            match tab.buffer.save() {
                Ok(()) => {
                    tab.status_message = "Saved".into();
                    if auto {
                        let path = tab.buffer.path().map(|p| p.to_path_buf());
                        if let Some(ref p) = path {
                            tab.compiler.compile(p);
                        }
                    }
                }
                Err(e) => {
                    tab.error_message =
                        Some(format!("Failed to save: {}", e));
                }
            }
        } else {
            self.save_as_file();
        }
    }

    fn save_as_file(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("LaTeX", &["tex"])
            .set_file_name("document.tex")
            .save_file()
        {
            let parent = path.parent().unwrap_or(Path::new("."));
            let file_name =
                path.file_name().unwrap_or(std::ffi::OsStr::new("document.tex"));
            let project_folder = parent.join(
                path.file_stem()
                    .unwrap_or(std::ffi::OsStr::new("document")),
            );
            let tex_path = project_folder.join(file_name);

            let auto = self.auto_compile;
            let tab = self.active_tab_mut();
            match tab.buffer.save_as(&tex_path) {
                Ok(()) => {
                    tab.title = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Untitled")
                        .to_string();
                    tab.status_message =
                        format!("Saved as {}", tex_path.display());
                    if auto {
                        tab.compiler.compile(&tex_path);
                    }
                }
                Err(e) => {
                    tab.error_message =
                        Some(format!("Failed to save: {}", e));
                }
            }
        }
    }
}

// --- Status bar ---
impl App {
    fn status_bar(&mut self, ui: &mut egui::Ui) {
        if self.tabs.is_empty() {
            ui.horizontal(|ui| {
                ui.colored_label(Color32::GRAY, "Ready");
            });
            return;
        }

        let tab = self.active_tab();
        let (line, col) = tab.buffer.cursor_line_col();
        let path = tab
            .buffer
            .path()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "Untitled".into());
        let dirty = if tab.buffer.dirty { " •" } else { "" };
        let compile_status = match tab.compiler.status() {
            CompileStatus::Idle => "",
            CompileStatus::Running => " | Compiling…",
            CompileStatus::Success => " | OK",
            CompileStatus::Failed => " | Error",
        };

        ui.horizontal(|ui| {
            ui.colored_label(
                Color32::GRAY,
                format!(
                    "{}{} | Ln {}, Col {} | {}{}",
                    path, dirty, line, col + 1, tab.status_message, compile_status,
                ),
            );
        });
    }
}

// --- Preview panel ---
impl App {
    fn preview_panel(&mut self, ui: &mut egui::Ui) {
        if self.tabs.is_empty() {
            return;
        }

        let show_error = self.active_tab().preview.render_error.is_some();
        let err_text = self.active_tab().preview.render_error.clone();
        let has_pdf_path = self.active_tab().preview.last_pdf_path.is_some();
        let zoom = self.active_tab().preview.zoom;
        let page = self.active_tab().preview.page;
        let tex = self.active_tab().preview_texture.clone();
        let num_pages = self.active_tab().preview.num_pages;
        let mut scroll_offset = self.active_tab().scroll_offset;

        let mut do_render = false;
        let mut new_zoom = zoom;
        let mut new_page = page;
        let mut open_externally = false;
        let mut pending_scroll: Option<egui::Vec2> = None;

        // Ctrl + scroll wheel to zoom preview
        if tex.is_some() {
            let ctrl = ui.input(|i| i.modifiers.ctrl);
            if ctrl {
                let scroll_delta = ui.input(|i| i.smooth_scroll_delta);
                if scroll_delta.y != 0.0 {
                    ui.ctx().input_mut(|i| i.smooth_scroll_delta = egui::Vec2::ZERO);
                    new_zoom = (zoom - scroll_delta.y / 60.0 * 0.25).max(0.25).min(4.0);
                    do_render = true;
                }
            }
        }

        let inner = ui.vertical(|ui| {
            // Toolbar
            ui.horizontal(|ui| {
                if ui.button("−").clicked() {
                    new_zoom = (zoom - 0.25).max(0.25);
                    do_render = true;
                }
                ui.label(format!("{}%", (zoom * 100.0) as u32));
                if ui.button("+").clicked() {
                    new_zoom = (zoom + 0.25).min(4.0);
                    do_render = true;
                }
                ui.separator();
                let has_prev = page > 0;
                if ui.add_enabled(has_prev, egui::Button::new("\u{25C0}")).clicked() {
                    new_page = page - 1;
                    do_render = true;
                }
                ui.label(format!("Page {}", page + 1));
                let has_next = num_pages.map_or(true, |n| page + 1 < n);
                if ui.add_enabled(has_next, egui::Button::new("\u{25B6}")).clicked() {
                    new_page = page + 1;
                    do_render = true;
                }
                ui.separator();
                if ui.button("\u{25C0} ").on_hover_text("Left").clicked() {
                    pending_scroll = Some(egui::vec2(-50.0, 0.0));
                }
                if ui.button("\u{25B6} ").on_hover_text("Right").clicked() {
                    pending_scroll = Some(egui::vec2(50.0, 0.0));
                }
                if ui.button("\u{25B2}").on_hover_text("Up").clicked() {
                    pending_scroll = Some(egui::vec2(0.0, -50.0));
                }
                if ui.button("\u{25BC}").on_hover_text("Down").clicked() {
                    pending_scroll = Some(egui::vec2(0.0, 50.0));
                }
            });
            ui.separator();

            if show_error {
                if let Some(err) = &err_text {
                    ui.colored_label(Color32::RED, err);
                }
                if has_pdf_path {
                    ui.add_space(8.0);
                    if ui.button("Open PDF Externally").clicked() {
                        open_externally = true;
                    }
                }
                return;
            }

            if let Some(tex) = tex {
                let img_size = tex.size_vec2();
                let image = Image::from_texture(
                    egui::load::SizedTexture::new(tex.id(), img_size),
                );

                scroll_offset += pending_scroll.unwrap_or_default();

                let output = ScrollArea::both()
                    .scroll_source(egui::containers::scroll_area::ScrollSource::MOUSE_WHEEL)
                    .vertical_scroll_offset(scroll_offset.y)
                    .horizontal_scroll_offset(scroll_offset.x)
                    .show(ui, |ui| {
                        ui.set_min_size(img_size);
                        ui.add(image);
                    });

                scroll_offset = output.state.offset;
                let _ = output.inner;
            } else {
                let avail = ui.available_size();
                let (_, resp) = ui.allocate_exact_size(avail, egui::Sense::hover());
                ui.painter().text(
                    resp.rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "No preview available",
                    egui::FontId::proportional(16.0),
                    Color32::GRAY,
                );
            }
        });
        let preview_rect = inner.response.rect;

        // Middle-mouse panning
        let (hover_pos, middle_down, delta) = ui.input(|i| {
            (i.pointer.hover_pos(), i.pointer.middle_down(), i.pointer.delta())
        });
        if let Some(pos) = hover_pos {
            if preview_rect.contains(pos) && middle_down {
                scroll_offset += delta;
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            }
        }

        self.active_tab_mut().scroll_offset = scroll_offset;

        if open_externally {
            self.active_tab_mut().preview.open_externally();
        }

        if do_render {
            let tab = self.active_tab_mut();
            tab.preview.zoom = new_zoom;
            tab.preview.page = new_page;
            if let Some(path) = tab.preview.last_pdf_path.clone() {
                tab.preview.render_pdf(&path, new_page);
            }
        }
    }
}

// --- Editor area ---
impl App {
    fn editor_area(&mut self, ui: &mut egui::Ui) {
        if self.tabs.is_empty() {
            return;
        }
        let line_count = self.active_tab().buffer.line_count();
        let gutter_width = if line_count > 0 {
            let digits = line_count.to_string().len();
            (digits as f32 * 9.0 + 16.0).max(36.0)
        } else {
            36.0
        };

        ui.horizontal_top(|ui| {
            let height = ui.available_height();
            let (gutter_rect, _) = ui.allocate_exact_size(
                egui::Vec2::new(gutter_width, height),
                egui::Sense::hover(),
            );
            self.paint_gutter(ui, gutter_rect, line_count);

            self.text_edit_area(ui);
        });
    }

    fn paint_gutter(&self, ui: &egui::Ui, rect: egui::Rect, line_count: usize) {
        let painter = ui.painter_at(rect);
        let bg = if self.theme == Theme::Dark {
            Color32::from_rgb(30, 34, 40)
        } else {
            Color32::from_rgb(240, 240, 244)
        };
        painter.rect_filled(rect, 0.0, bg);

        let sep_x = rect.right() - 1.0;
        let sep_color = if self.theme == Theme::Dark {
            Color32::from_rgb(48, 54, 62)
        } else {
            Color32::from_rgb(210, 210, 215)
        };
        painter.line_segment(
            [
                egui::pos2(sep_x, rect.top()),
                egui::pos2(sep_x, rect.bottom()),
            ],
            egui::Stroke::new(1.0, sep_color),
        );

        let text_color = if self.theme == Theme::Dark {
            Color32::from_rgb(100, 110, 130)
        } else {
            Color32::from_rgb(150, 155, 165)
        };
        let font_id = FontId::monospace(12.0);

        let line_height = 16.0;
        let start_y = rect.top() + 4.0;

        for i in 1..=line_count {
            let y = start_y + (i - 1) as f32 * line_height;
            if y > rect.bottom() {
                break;
            }
            painter.text(
                egui::pos2(rect.right() - 6.0, y),
                egui::Align2::RIGHT_TOP,
                &i.to_string(),
                font_id.clone(),
                text_color,
            );
        }
    }

    fn text_edit_area(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme;

        let tab = self.active_tab_mut();
        let mut text = std::mem::take(&mut tab.buffer.text);
        let mut layouter =
            |layouter_ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap_width: f32| {
                let text = buf.as_str();
                let tokens = lexer::tokenize(text);

                let (text_color, cmd_color, math_color, brace_color, comment_color) =
                    match theme {
                        Theme::Dark => (
                            Color32::from_rgb(220, 220, 224),
                            Color32::from_rgb(86, 156, 214),
                            Color32::from_rgb(214, 157, 133),
                            Color32::from_rgb(255, 215, 0),
                            Color32::from_rgb(106, 153, 85),
                        ),
                        Theme::Light => (
                            Color32::from_rgb(30, 30, 34),
                            Color32::from_rgb(0, 56, 168),
                            Color32::from_rgb(196, 86, 4),
                            Color32::from_rgb(180, 120, 0),
                            Color32::from_rgb(0, 128, 0),
                        ),
                    };

                let job = egui::text::LayoutJob {
                    text: text.into(),
                    sections: tokens
                        .iter()
                        .map(|token| {
                            let color = match token.token_type {
                                lexer::TokenType::Command => cmd_color,
                                lexer::TokenType::MathDollar
                                | lexer::TokenType::MathDoubleDollar => math_color,
                                lexer::TokenType::OpenBrace
                                | lexer::TokenType::CloseBrace => brace_color,
                                lexer::TokenType::Comment => comment_color,
                                lexer::TokenType::Text => text_color,
                            };
                            egui::text::LayoutSection {
                                leading_space: 0.0,
                                byte_range: token.start..token.end,
                                format: egui::text::TextFormat {
                                    font_id: FontId::monospace(14.0),
                                    color,
                                    ..Default::default()
                                },
                            }
                        })
                        .collect(),
                    wrap: egui::text::TextWrapping {
                        max_width: wrap_width,
                        ..Default::default()
                    },
                    ..Default::default()
                };

                layouter_ui.fonts_mut(|f| f.layout_job(job))
            };

        let response = ui.add_sized(
            ui.available_size(),
            TextEdit::multiline(&mut text)
                .code_editor()
                .desired_width(f32::INFINITY)
                .layouter(&mut layouter),
        );

        // Update buffer text and cursor from egui state
        let cursor_char = egui::TextEdit::load_state(ui.ctx(), response.id)
            .and_then(|state| state.cursor.char_range())
            .map_or(0, |range| range.primary.index);
        let cursor_pos = text
            .char_indices()
            .nth(cursor_char)
            .map_or(text.len(), |(b, _)| b);
        let changed = response.changed();
        let tab = self.active_tab_mut();
        tab.buffer.text = text;
        tab.buffer.cursor = cursor_pos;
        if changed {
            tab.buffer.sync_after_edit();
        }

        // --- Autocomplete ---
        self.completion_visible = false;
        if response.has_focus() && cursor_pos > 0 {
            let text = &self.active_tab().buffer.text;
            let cursor = cursor_pos.min(text.len());
            let before = &text[..cursor];

            if let Some(bslash) = before.rfind('\\') {
                let partial = &before[bslash..];
                if partial.len() > 1
                    && partial[1..].chars().all(|c| c.is_alphanumeric())
                {
                    let matches = completions::find_completions(partial);
                    if !matches.is_empty() {
                        self.completion_visible = true;
                        self.completion_matches =
                            matches.into_iter().map(|s| s.to_string()).collect();
                        self.completion_byte_range = Some((bslash, cursor));
                    }
                }
            }
        }

        // Render completion popup (no borrow of self in the closure)
        if self.completion_visible {
            let line_height = 16.0;
            let (cursor_line, _) = self.active_tab().buffer.cursor_line_col();
            let popup_pos = egui::pos2(
                response.rect.left() + 4.0,
                response.rect.top() + (cursor_line as f32) * line_height + line_height,
            );
            let matches = self.completion_matches.clone();
            let range = self.completion_byte_range;
            let bg_fill = if self.theme == Theme::Dark {
                Color32::from_rgb(40, 44, 52)
            } else {
                Color32::from_rgb(255, 255, 255)
            };

            let mut close = false;
            let mut selected: Option<usize> = None;
            let popup_id = egui::Id::new("latex_completions");

            let _ = egui::Area::new(popup_id)
                .fixed_pos(popup_pos)
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    let mut style = (*ui.ctx().global_style()).clone();
                    style.visuals.widgets.noninteractive.bg_fill = bg_fill;
                    ui.set_style(style);

                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.set_min_width(180.0);
                        ui.set_max_height(300.0);
                        ScrollArea::vertical()
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                for (i, cmd) in matches.iter().enumerate() {
                                    let label = cmd.replacen('\\', "", 1);
                                    if ui.button(&label).clicked() {
                                        close = true;
                                        selected = Some(i);
                                    }
                                }
                            });
                    });
                });

            if close {
                if let Some(idx) = selected {
                    if let Some(replacement) = matches.get(idx) {
                        if let Some((start, end)) = range {
                            let tab = self.active_tab_mut();
                            let text = &mut tab.buffer.text;
                            if start <= end && end <= text.len() {
                                text.replace_range(start..end, replacement);
                                tab.buffer.cursor = start + replacement.len();
                                tab.buffer.sync_after_edit();
                            }
                        }
                    }
                }
                self.completion_visible = false;
            }
        }
    }
}
