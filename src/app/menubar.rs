use std::path::{Path, PathBuf};

use crate::app::App;
use crate::app::tab::Tab;
use crate::buffer::EditorBuffer;
use crate::types::Theme;

fn project_tex_path(path: &Path) -> PathBuf {
    let parent = path.parent().unwrap_or(Path::new("."));
    let file_name = path
        .file_name()
        .unwrap_or(std::ffi::OsStr::new("document.tex"));
    let project_folder = parent.join(
        path.file_stem()
            .unwrap_or(std::ffi::OsStr::new("document")),
    );
    project_folder.join(file_name)
}

impl App {
    pub(super) fn menu_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New Document").clicked() {
                    ui.close();
                    self.file_dialog_action =
                        Some(super::FileDialogAction::NewDocument);
                }
                if ui.button("Open…").clicked() {
                    ui.close();
                    self.file_dialog_action =
                        Some(super::FileDialogAction::Open);
                }
                let has_tabs = !self.tabs.is_empty();
                if ui.add_enabled(has_tabs, egui::Button::new("Save")).clicked() {
                    ui.close();
                    self.file_dialog_action =
                        Some(super::FileDialogAction::Save);
                }
                if ui.add_enabled(has_tabs, egui::Button::new("Save As…")).clicked() {
                    ui.close();
                    self.file_dialog_action =
                        Some(super::FileDialogAction::SaveAs);
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
                ui.menu_button("Build", |_ui| {});
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
                ui.menu_button("Theme", |ui| {
                    if ui
                        .selectable_value(&mut self.theme, Theme::System, "System")
                        .clicked()
                    {
                        ui.close();
                        self.apply_theme(ui.ctx().clone());
                        ui.ctx().request_repaint();
                    }
                    if ui
                        .selectable_value(&mut self.theme, Theme::Light, "Light")
                        .clicked()
                    {
                        ui.close();
                        self.apply_theme(ui.ctx().clone());
                        ui.ctx().request_repaint();
                    }
                    if ui
                        .selectable_value(&mut self.theme, Theme::Dark, "Dark")
                        .clicked()
                    {
                        ui.close();
                        self.apply_theme(ui.ctx().clone());
                        ui.ctx().request_repaint();
                    }
                });
            });
        });
    }

    pub(super) fn trigger_compile(&mut self) {
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

    pub(super) fn new_document(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("LaTeX", &["tex"])
            .set_file_name("document.tex")
            .save_file()
        {
            let tex_path = project_tex_path(&path);

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

    pub(super) fn open_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("LaTeX", &["tex", "sty", "cls"])
            .pick_file()
        {
            let tab = Tab::load(&path);
            self.tabs.push(tab);
            self.active_tab = self.tabs.len() - 1;
        }
    }

    pub(super) fn save_file(&mut self) {
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

    pub(super) fn save_as_file(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("LaTeX", &["tex"])
            .set_file_name("document.tex")
            .save_file()
        {
            let tex_path = project_tex_path(&path);

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
