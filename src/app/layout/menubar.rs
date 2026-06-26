use std::path::{Path, PathBuf};

use crate::app::App;
use crate::app::tab::Tab;
use crate::buffer::EditorBuffer;
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
    pub(crate) fn menu_bar(&mut self, ui: &mut egui::Ui) {
        ui.style_mut()
            .text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(13.0));
        ui.style_mut()
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(13.0));
        ui.style_mut().spacing.button_padding = egui::vec2(16.0, 10.0);
        ui.style_mut().spacing.item_spacing = egui::vec2(4.0, 0.0);

        ui.add_space(4.0);
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New Document").clicked() {
                    ui.close();
                    self.file_dialog_action =
                        Some(crate::app::FileDialogAction::NewDocument);
                }
                if ui.button("Open…").clicked() {
                    ui.close();
                    self.file_dialog_action =
                        Some(crate::app::FileDialogAction::Open);
                }
                let has_tabs = !self.tabs.is_empty();
                if ui.add_enabled(has_tabs, crate::components::button::standard("Save")).clicked() {
                    ui.close();
                    self.file_dialog_action =
                        Some(crate::app::FileDialogAction::Save);
                }
                if ui.add_enabled(has_tabs, crate::components::button::standard("Save As…")).clicked() {
                    ui.close();
                    self.file_dialog_action =
                        Some(crate::app::FileDialogAction::SaveAs);
                }
                ui.separator();
                if ui.button("Quit").clicked() {
                    ui.close();
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });

            let has_tabs = !self.tabs.is_empty();
            ui.menu_button("View", |ui| {
                if ui.add_enabled(has_tabs, crate::components::button::standard("Toggle Preview")).clicked() {
                    ui.close();
                    let tab = self.active_tab_mut();
                    tab.show_preview = !tab.show_preview;
                    tab.status_message = if tab.show_preview {
                        "Preview shown".into()
                    } else {
                        "Preview hidden".into()
                    };
                }
                let fs = ui.ctx().input(|i| i.viewport().fullscreen.unwrap_or(false));
                ui.scope(|ui| {
                    if fs && ui.visuals().dark_mode {
                        ui.visuals_mut().selection.bg_fill = egui::Color32::from_rgb(60, 60, 60);
                    }
                    if ui.add(crate::components::button::standard("\u{26F6} Fullscreen    F12").selected(fs)).clicked() {
                        ui.close();
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Fullscreen(!fs));
                    }
                });
            });

            ui.menu_button("Options", |ui| {
                ui.set_min_width(220.0);
                
                ui.menu_button("Theme", |ui| {
                    if ui.radio_value(&mut self.theme, crate::types::Theme::System, "System").clicked() {
                        crate::app::App::save_theme(self.theme);
                        ui.close();
                    }
                    if ui.radio_value(&mut self.theme, crate::types::Theme::Light, "Light").clicked() {
                        crate::app::App::save_theme(self.theme);
                        ui.close();
                    }
                    if ui.radio_value(&mut self.theme, crate::types::Theme::Dark, "Dark").clicked() {
                        crate::app::App::save_theme(self.theme);
                        ui.close();
                    }
                });
                
                if ui.button("Integrate LLM").clicked() {
                    self.show_llm_settings = true;
                    ui.close();
                }
            });

            ui.menu_button("Help", |ui| {
                ui.set_min_width(220.0);
                if ui.button("Check for Updates").clicked() {
                    self.check_for_updates(ui.ctx().clone());
                    ui.close();
                }
                if ui.button("About").clicked() {
                    self.about_open = true;
                    ui.close();
                }
            });
        });
        ui.add_space(4.0);
    }

    pub(crate) fn trigger_compile(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        
        let mut cmd = std::process::Command::new("pdflatex");
        cmd.arg("--version");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
        
        let is_missing = match cmd.output() {
            Ok(output) => !output.status.success(),
            Err(e) => e.kind() == std::io::ErrorKind::NotFound,
        };
        
        if is_missing {
            self.show_compiler_dialog = true;
            return;
        }

        let tab = self.active_tab_mut();
        tab.show_preview = true;
        let path = tab.buffer.path().map(|p| p.to_path_buf());
        if let Some(ref path) = path {
            if let Err(e) = std::fs::write(path, &tab.buffer.text) {
                tab.error_message =
                    Some(format!("Failed to write file for compile: {}", e));
                return;
            }
            tab.compiler.compile(path);
        } else {
            tab.error_message =
                Some("Please save the file before compiling.".into());
        }
    }

    pub(crate) fn trigger_llm_correction(&mut self) {
        self.do_llm_correction(None);
    }

    pub(crate) fn trigger_llm_correction_for_line(&mut self, line: usize) {
        self.do_llm_correction(Some(line));
    }

    fn do_llm_correction(&mut self, target_line: Option<usize>) {
        if self.tabs.is_empty() || self.llm_correction_in_progress {
            return;
        }
        self.llm_correction_in_progress = true;
        let tab = self.active_tab_mut();
        tab.ai_output_log.clear();
        
        let mut text = tab.buffer.text.clone();
        if let Some(l) = target_line {
            if let Some(line_text) = text.lines().nth(l.saturating_sub(1)) {
                text = line_text.to_string();
            }
        }
        
        let errors = tab.error_message.clone().unwrap_or_default();
        let tx = self.llm_tx.clone();
        let api_key = self.llm_api_key.clone();
        
        std::thread::spawn(move || {
            let prompt = if target_line.is_some() {
                format!(
                    "Fix any LaTeX syntax errors in the following line based on these errors: {}\n\nLine:\n{}\n\nReturn a single JSON object (NOT an array): if the error EXPLICITLY states a missing .sty file, use {{\"action\":\"install\",\"package\":\"name\",\"explanation\":\"short explanation\"}}. Otherwise, if correcting code, use {{\"action\":\"correct\",\"text\":\"corrected line\",\"explanation\":\"short explanation\"}}. Do not guess missing packages for undefined control sequences; fix the code instead. If correcting, provide the FULL corrected line WITHOUT adding any new comments to the code. Keep the explanation short to save tokens. Return ONLY the JSON.",
                    errors, text
                )
            } else {
                format!(
                    "Fix any LaTeX syntax errors in the following document based on these errors: {}\n\nDocument:\n{}\n\nReturn a single JSON object (NOT an array): if the error EXPLICITLY states a missing .sty file, use {{\"action\":\"install\",\"package\":\"name\",\"explanation\":\"short explanation\"}}. Otherwise, if correcting code, use {{\"action\":\"correct\",\"text\":\"corrected document\",\"explanation\":\"short explanation\"}}. Do not guess missing packages for undefined control sequences; fix the code instead. If correcting, you MUST provide the ENTIRE document with the corrections applied, do not just return the corrected snippet. DO NOT add any new comments to the code explaining your changes. Keep the explanation short to save tokens. Return ONLY the JSON.",
                    errors, text
                )
            };
            
            let body = serde_json::to_string(&serde_json::json!({
                "model": "gpt-4o-mini",
                "messages": [
                    {"role": "user", "content": prompt}
                ]
            })).unwrap_or_default();

            let request = ureq::post("https://api.openai.com/v1/chat/completions")
                .header("Authorization", &format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .send(body);
                
            match request {
                Ok(response) => {
                    use std::io::Read;
                    let mut reader = response.into_body().into_reader();
                    let mut body_str = String::new();
                    let _ = reader.read_to_string(&mut body_str);
                    
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body_str) {
                        if let Some(content) = json["choices"][0]["message"]["content"].as_str() {
                            let content = content.trim();
                            let parsed = if content.starts_with("```json") {
                                let c = content.trim_start_matches("```json").trim_end_matches("```").trim();
                                serde_json::from_str::<serde_json::Value>(c)
                            } else {
                                serde_json::from_str::<serde_json::Value>(content)
                            };

                            if let Ok(mut obj) = parsed {
                                if let Some(arr) = obj.as_array() {
                                    if let Some(first) = arr.first() {
                                        obj = first.clone();
                                    }
                                }
                                let explanation = obj.get("explanation").and_then(|v| v.as_str()).map(|s| s.to_string());
                                if let Some(action) = obj.get("action").and_then(|v| v.as_str()) {
                                    if action == "install" {
                                        if let Some(pkg) = obj.get("package").and_then(|v| v.as_str()) {
                                            let _ = tx.send(Ok(crate::app::LlmAction::InstallPackage {
                                                package: pkg.to_string(),
                                                explanation,
                                            }));
                                            return;
                                        }
                                    } else if action == "correct" {
                                        if let Some(corrected_text) = obj.get("text").and_then(|v| v.as_str()) {
                                            let _ = tx.send(Ok(crate::app::LlmAction::Correction {
                                                text: corrected_text.to_string(),
                                                line: target_line,
                                                explanation,
                                            }));
                                            return;
                                        }
                                    }
                                }
                            }
                            let _ = tx.send(Err(format!("Invalid response format from LLM. Raw response: {}", content)));
                        } else {
                            let _ = tx.send(Err("No content in LLM response".to_string()));
                        }
                    } else {
                        let _ = tx.send(Err("Failed to parse LLM response".to_string()));
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(format!("LLM network error: {}", e)));
                }
            }
        });
    }

    pub(crate) fn new_document(&mut self) {
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

    pub(crate) fn open_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("LaTeX", &["tex", "sty", "cls"])
            .pick_file()
        {
            if let Some(existing) = self.tabs.iter().position(|t| t.buffer.path() == Some(&path)) {
                self.active_tab = existing;
                return;
            }
            let tab = Tab::load(&path);
            self.tabs.push(tab);
            self.active_tab = self.tabs.len() - 1;
        }
    }

    pub(crate) fn save_file(&mut self) {
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

    pub(crate) fn save_as_file(&mut self) {
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
