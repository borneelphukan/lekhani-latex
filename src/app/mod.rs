pub mod tab;
pub mod layout;
mod editor;

use std::path::PathBuf;
use std::time::Instant;


use egui::{CentralPanel, Color32, Panel};
use std::sync::OnceLock;
use regex::Regex;

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

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

#[derive(Clone)]
enum CompilerDownloadState {
    None,
    Downloading(f32),
    Complete,
    Failed(String),
}

enum UpdateMessage {
    CheckResult(bool, Option<String>),
    DownloadProgress(f32),
    DownloadComplete(Result<String, String>),
    CompilerDownloadProgress(f32),
    CompilerDownloadComplete(Result<PathBuf, String>),
    #[allow(dead_code)]
    OsThemeChanged(egui::Theme),
    PackageInstalled(String, Result<(), String>),
}

#[derive(Clone, Debug)]
pub enum LlmAction {
    Correction { text: String, line: Option<usize>, explanation: Option<String> },
    InstallPackage { package: String, explanation: Option<String> },
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum OutputPanelTab {
    Compiler,
    AI,
}

pub struct App {
    tabs: Vec<tab::Tab>,
    active_tab: usize,
    theme: Theme,
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
    auto_compile: bool,
    last_auto_compile: Option<Instant>,
    last_content_change: Option<Instant>,
    tab_drag: Option<(usize, f32)>,
    llm_correction_in_progress: bool,
    llm_tx: std::sync::mpsc::Sender<Result<LlmAction, String>>,
    llm_rx: std::sync::mpsc::Receiver<Result<LlmAction, String>>,
    llm_api_key: String,
    has_saved_llm_key: bool,
    llm_provider: String,
    llm_api_key_error: Option<String>,
    show_llm_settings: bool,
    os_theme: Option<egui::Theme>,
    startup_compiler_checked: bool,
    show_compiler_dialog: bool,
    compiler_download_state: CompilerDownloadState,
    output_panel_tab: OutputPanelTab,
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
        let (llm_tx, llm_rx) = std::sync::mpsc::channel::<Result<LlmAction, String>>();
        let mut app = Self {
            tabs: Vec::new(),
            active_tab: 0,
            theme: Theme::System,
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
            auto_compile: false,
            last_auto_compile: None,
            last_content_change: None,
            tab_drag: None,
            llm_correction_in_progress: false,
            llm_tx,
            llm_rx,
            llm_api_key: String::new(),
            has_saved_llm_key: false,
            llm_provider: "OpenAI".to_string(),
            llm_api_key_error: None,
            show_llm_settings: false,
            os_theme: None,
            startup_compiler_checked: false,
            show_compiler_dialog: false,
            compiler_download_state: CompilerDownloadState::None,
            output_panel_tab: OutputPanelTab::Compiler,
        };

        let (initial_key, has_saved) = Self::load_initial_api_key();
        app.llm_api_key = initial_key;
        app.has_saved_llm_key = has_saved;
        app.theme = Self::load_theme();

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

        #[cfg(target_os = "linux")]
        {
            let tx = app.update_tx.clone();
            let ctx = cc.egui_ctx.clone();
            std::thread::spawn(move || {
                let mut last_theme = None;
                loop {
                    if let Ok(output) = std::process::Command::new("gsettings")
                        .args(&["get", "org.gnome.desktop.interface", "color-scheme"])
                        .output()
                    {
                        let s = String::from_utf8_lossy(&output.stdout);
                        let current = if s.contains("prefer-dark") {
                            Some(egui::Theme::Dark)
                        } else if s.contains("prefer-light") || s.contains("default") {
                            Some(egui::Theme::Light)
                        } else {
                            None
                        };
                        
                        if current != last_theme {
                            last_theme = current;
                            if let Some(t) = current {
                                let _ = tx.send(UpdateMessage::OsThemeChanged(t));
                                ctx.request_repaint();
                            }
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_secs(2));
                }
            });
        }

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

    fn get_llm_config_path() -> Option<std::path::PathBuf> {
        dirs::config_dir().map(|mut p| {
            p.push("lekhani-latex");
            p.push("llm_config.json");
            p
        })
    }

    fn get_app_config_path() -> Option<std::path::PathBuf> {
        dirs::config_dir().map(|mut p| {
            p.push("lekhani-latex");
            p.push("config.json");
            p
        })
    }

    fn read_app_config() -> serde_json::Value {
        let mut config = serde_json::json!({});
        if let Some(path) = Self::get_app_config_path() {
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(json) = serde_json::from_str(&content) {
                        config = json;
                    }
                }
            } else if let Some(old_path) = Self::get_llm_config_path() {
                if old_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&old_path) {
                        if let Ok(json) = serde_json::from_str(&content) {
                            config = json;
                            if let Some(parent) = path.parent() {
                                let _ = std::fs::create_dir_all(parent);
                            }
                            let _ = std::fs::write(&path, serde_json::to_string_pretty(&config).unwrap_or_default());
                            let _ = std::fs::remove_file(old_path);
                        }
                    }
                }
            }
        }
        config
    }

    fn write_app_config(config: &serde_json::Value) {
        if let Some(path) = Self::get_app_config_path() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(path, serde_json::to_string_pretty(config).unwrap_or_default());
        }
    }

    fn load_llm_api_key() -> Option<String> {
        let config = Self::read_app_config();
        if let Some(key) = config.get("llm_api_key").and_then(|v| v.as_str()) {
            if !key.is_empty() {
                return Some(key.to_string());
            }
        }
        None
    }

    fn load_initial_api_key() -> (String, bool) {
        if let Some(key) = Self::load_llm_api_key() {
            return (key, true);
        }
        if let Ok(key) = std::env::var("LLM_API_KEY") {
            if !key.is_empty() {
                return (key, true);
            }
        }
        (String::new(), false)
    }

    fn save_llm_api_key(key: &str) {
        let mut config = Self::read_app_config();
        config["llm_api_key"] = serde_json::json!(key);
        Self::write_app_config(&config);
    }

    fn remove_llm_api_key() {
        let mut config = Self::read_app_config();
        if let Some(obj) = config.as_object_mut() {
            obj.remove("llm_api_key");
        }
        Self::write_app_config(&config);
    }

    pub fn save_theme(theme: Theme) {
        let mut config = Self::read_app_config();
        let theme_str = match theme {
            Theme::System => "System",
            Theme::Light => "Light",
            Theme::Dark => "Dark",
        };
        config["theme"] = serde_json::json!(theme_str);
        Self::write_app_config(&config);
    }

    fn load_theme() -> Theme {
        let config = Self::read_app_config();
        if let Some(theme_str) = config.get("theme").and_then(|v| v.as_str()) {
            match theme_str {
                "System" => Theme::System,
                "Light" => Theme::Light,
                "Dark" => Theme::Dark,
                _ => Theme::System,
            }
        } else {
            Theme::System
        }
    }

    fn active_tab(&self) -> &tab::Tab {
        &self.tabs[self.active_tab]
    }

    fn active_tab_mut(&mut self) -> &mut tab::Tab {
        &mut self.tabs[self.active_tab]
    }

    fn apply_theme(&self, ctx: egui::Context) {
        let is_dark = match self.theme {
            crate::types::Theme::Dark => true,
            crate::types::Theme::Light => false,
            crate::types::Theme::System => {
                let sys = self.os_theme.or_else(|| ctx.system_theme());
                sys == Some(egui::Theme::Dark) || sys.is_none()
            }
        };
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
            style.visuals.panel_fill = egui::Color32::from_rgb(240, 240, 240);
            style.visuals.window_fill = egui::Color32::from_rgb(245, 245, 245);
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
                        tab.show_preview = true;
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
                        tab.preview.rendered_pages.clear();
                        tab.preview.active_renders.clear();
                        tab.preview_textures.clear();
                        tab.preview.ensure_page_rendered(&pdf_path, 0);
                        self.show_outputs_requested = true;
                    }
                    CompileEvent::Failure(errors) => {
                        tab.compile_start_time.take();
                        tab.status_message = "Compilation failed".into();
                        let error_text = errors.join("\n");
                        tab.error_message = Some(error_text.clone());
                        tab.output_log.clear();

                        if error_text.contains("was not found. Is a LaTeX distribution (MiKTeX/TeX Live) installed?") {
                            self.show_compiler_dialog = true;
                        }
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
                    PreviewEvent::NewImage(_page, _color_image) => {
                        // Handled by update_preview_textures because it needs context
                    }
                    PreviewEvent::Error(_page, e) => {
                        // For simplicity, just log or set error if the currently viewed page failed
                        tab.preview.render_error = Some(e);
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

        while let Ok(result) = self.llm_rx.try_recv() {
            self.llm_correction_in_progress = false;
            let mut set_ai_tab = false;
            if !self.tabs.is_empty() {
                let tab = self.active_tab_mut();
                match result {
                    Ok(action) => match action {
                        LlmAction::Correction { text, line, explanation } => {
                            if let Some(expl) = explanation {
                                tab.ai_output_log.push((expl, egui::Color32::from_rgb(100, 200, 100)));
                            } else {
                                tab.ai_output_log.push(("Correction applied successfully.".to_string(), egui::Color32::from_rgb(100, 200, 100)));
                            }
                            if let Some(l) = line {
                                let lines: Vec<&str> = tab.buffer.text.lines().collect();
                                if l > 0 && l <= lines.len() {
                                    let mut new_text = String::new();
                                    for (i, &old_line) in lines.iter().enumerate() {
                                        if i == l - 1 {
                                            new_text.push_str(&text);
                                        } else {
                                            new_text.push_str(old_line);
                                        }
                                        new_text.push('\n');
                                    }
                                    if new_text.ends_with('\n') && !tab.buffer.text.ends_with('\n') {
                                        new_text.pop();
                                    }
                                    tab.buffer.replace_all(&new_text);
                                }
                            } else {
                                tab.buffer.replace_all(&text);
                            }
                            tab.status_message = "Syntax corrected via LLM".into();
                            set_ai_tab = true;
                        }
                        LlmAction::InstallPackage { package, explanation } => {
                            let pkg = package;
                            if let Some(expl) = explanation {
                                tab.ai_output_log.push((expl, egui::Color32::from_rgb(100, 200, 100)));
                            }
                            tab.ai_output_log.push((format!("Attempting to install package {}...", pkg), egui::Color32::from_rgb(200, 200, 100)));
                            tab.status_message = format!("Installing package {}...", pkg).into();
                            let tx = self.update_tx.clone();
                            std::thread::spawn(move || {
                                let output = std::process::Command::new("tlmgr")
                                    .arg("install")
                                    .arg(&pkg)
                                    .output();
                                let result = match output {
                                    Ok(o) => {
                                        if o.status.success() {
                                            Ok(())
                                        } else {
                                            Err(String::from_utf8_lossy(&o.stderr).to_string())
                                        }
                                    }
                                    Err(e) => Err(e.to_string()),
                                };
                                let _ = tx.send(UpdateMessage::PackageInstalled(pkg, result));
                            });
                            set_ai_tab = true;
                        }
                    },
                    Err(err) => {
                        tab.error_message = Some(err.clone());
                        tab.output_log.clear();
                        tab.output_log.push((
                            format!("× LLM Error: {}", err),
                            Color32::from_rgb(220, 60, 60),
                        ));
                        set_ai_tab = true;
                    }
                }
            }
            if set_ai_tab {
                self.output_panel_tab = OutputPanelTab::AI;
                self.show_outputs_requested = true;
                self.show_outputs = true;
            }
        }
    }

    fn update_preview_textures(&mut self, ctx: &egui::Context) {
        for tab in &mut self.tabs {
            for (page, img) in &tab.preview.rendered_pages {
                if !tab.preview_textures.contains_key(page) {
                    tab.preview_textures.insert(*page, ctx.load_texture(
                        &format!("preview_{}_{}", tab.title, page),
                        img.clone(),
                        egui::TextureOptions::LINEAR,
                    ));
                }
            }
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if !self.startup_compiler_checked {
            self.startup_compiler_checked = true;
            let mut cmd = std::process::Command::new("pdflatex");
            cmd.arg("--version");
            #[cfg(windows)]
            cmd.creation_flags(0x08000000);
            
            let is_missing = match cmd.output() {
                Ok(output) => !output.status.success(),
                Err(e) => e.kind() == std::io::ErrorKind::NotFound,
            };
            
            if is_missing {
                self.show_compiler_dialog = true;
            }
        }

        self.apply_theme(ui.ctx().clone());

        self.process_update_messages();

        let is_fullscreen = ui.ctx().input(|i| i.viewport().fullscreen.unwrap_or(false));
        if is_fullscreen && ui.ctx().input(|i| i.key_pressed(egui::Key::Escape)) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
        }
        if ui.ctx().input(|i| i.key_pressed(egui::Key::F12)) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Fullscreen(!is_fullscreen));
        }

        if ui.ctx().input(|i| i.viewport().close_requested()) {
            let has_unsaved = self.tabs.iter().any(|t| t.buffer.dirty);
            if has_unsaved {
                let res = rfd::MessageDialog::new()
                    .set_title("Unsaved Changes")
                    .set_description("You have unsaved documents. Are you sure you want to quit without saving?")
                    .set_buttons(rfd::MessageButtons::YesNo)
                    .show();
                if res != rfd::MessageDialogResult::Yes {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::CancelClose);
                }
            }
        }

        if ui.ctx().input(|i| i.modifiers.command && i.key_pressed(egui::Key::O)) {
            self.file_dialog_action = Some(FileDialogAction::Open);
        }

        if !self.tabs.is_empty() {
            if ui.ctx().input(|i| i.modifiers.command && i.key_pressed(egui::Key::S)) {
                self.file_dialog_action = Some(FileDialogAction::Save);
            }
            if ui.ctx().input(|i| i.modifiers.command && i.key_pressed(egui::Key::Z)) {
                self.active_tab_mut().buffer.undo();
            }
            if ui.ctx().input(|i| i.modifiers.command && i.key_pressed(egui::Key::Y)) {
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

        let show_panels = !is_fullscreen || cfg!(target_os = "macos");

        if show_panels {
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
        }

        if show_panels {
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
                    ui.scope(|ui| {
                        ui.style_mut().text_styles.insert(egui::TextStyle::Button, egui::FontId::proportional(16.0));
                        ui.spacing_mut().button_padding = egui::vec2(16.0, 8.0);
                        if ui.add(crate::components::button::standard("New Document")).clicked() {
                            self.new_document();
                        }
                        ui.add_space(4.0);
                        if ui.add(crate::components::button::standard("Open File")).clicked() {
                            self.open_file();
                        }
                    });
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

        if self.about_open {
            let mut about_open = self.about_open;
            let mut close_clicked = false;
            egui::Window::new("About")
                .open(&mut about_open)
                .resizable(false)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .default_size([700.0, 380.0])
                .min_size([700.0, 380.0])
                .show(ui.ctx(), |ui| {
                    ui.set_min_size(egui::vec2(700.0, 380.0));
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
                                ui.ctx().copy_text(version_info);
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

                        ui.add_space(16.0);
                        ui.horizontal(|ui| {
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button(egui::RichText::new("Close").size(14.0)).clicked() {
                                    close_clicked = true;
                                }
                            });
                        });
                    });
                });
            if close_clicked {
                about_open = false;
            }
            self.about_open = about_open;
        }

        self.ui_update_dialog(ui);
        self.llm_settings_dialog(ui.ctx());
        self.compiler_dialog_ui(ui.ctx());
    }

    fn on_exit(&mut self) {
        for tab in &mut self.tabs {
            // Release GPU textures before the rendering context is destroyed
            tab.preview_textures.clear();
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
                UpdateMessage::CheckResult(available, version) => {
                    if available {
                        self.update_state =
                            UpdateState::Prompt(version.unwrap_or_else(|| "Unknown".into()));
                    } else {
                        self.update_state = UpdateState::None;
                        rfd::MessageDialog::new()
                            .set_title("No Updates")
                            .set_description("No update available.")
                            .set_buttons(rfd::MessageButtons::Ok)
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
                            let mut c = std::process::Command::new(&path);
                            #[cfg(windows)]
                            c.creation_flags(CREATE_NO_WINDOW);
                            if let Err(_) = c.spawn() {
                                // Toast notification not implemented; update state already None
                            } else {
                                std::process::exit(0);
                            }
                        }
                        Err(_) => {}
                    }
                }
                UpdateMessage::CompilerDownloadProgress(progress) => {
                    self.compiler_download_state = CompilerDownloadState::Downloading(progress);
                }
                UpdateMessage::OsThemeChanged(theme) => {
                    self.os_theme = Some(theme);
                }
                UpdateMessage::PackageInstalled(pkg, result) => {
                    let mut set_ai_tab = false;
                    if !self.tabs.is_empty() {
                        let tab = self.active_tab_mut();
                        match result {
                            Ok(_) => {
                                tab.status_message = format!("Package {} installed successfully", pkg).into();
                                tab.ai_output_log.push((
                                    format!("Package {} installed successfully", pkg),
                                    egui::Color32::from_rgb(60, 180, 75),
                                ));
                            }
                            Err(e) => {
                                tab.status_message = format!("Failed to install {}", pkg).into();
                                tab.ai_output_log.push((
                                    format!("Failed to install {}: {}", pkg, e),
                                    egui::Color32::from_rgb(220, 60, 60),
                                ));
                            }
                        }
                        set_ai_tab = true;
                    }
                    if set_ai_tab {
                        self.output_panel_tab = OutputPanelTab::AI;
                        self.show_outputs_requested = true;
                    }
                }
                UpdateMessage::CompilerDownloadComplete(result) => {
                    match result {
                        Ok(path) => {
                            self.compiler_download_state = CompilerDownloadState::Complete;
                            
                            // Launch the installer
                            #[cfg(target_os = "windows")]
                            {
                                let _ = std::process::Command::new(&path).spawn();
                            }
                            #[cfg(target_os = "linux")]
                            {
                                // Provide a command to run or run it in terminal
                                // For tar.gz, we just show a message, or try to run it via terminal
                                let extract_dir = path.parent().unwrap_or(std::path::Path::new("/tmp"));
                                let cmd = format!(
                                    "cd {} && tar xzf {} && cd install-tl-* && sudo ./install-tl",
                                    extract_dir.display(),
                                    path.display()
                                );
                                
                                let terminals = ["gnome-terminal", "konsole", "xfce4-terminal", "alacritty", "xterm"];
                                for term in terminals {
                                    let mut spawn_cmd = std::process::Command::new(term);
                                    if term == "gnome-terminal" {
                                        spawn_cmd.args(&["--", "bash", "-c", &format!("{}; read -p '\nPress enter to close...'", cmd)]);
                                    } else {
                                        spawn_cmd.args(&["-e", "bash", "-c", &format!("{}; read -p '\nPress enter to close...'", cmd)]);
                                    }
                                    
                                    if spawn_cmd.spawn().is_ok() {
                                        break;
                                    }
                                }
                            }
                            #[cfg(target_os = "macos")]
                            {
                                let _ = std::process::Command::new("open").arg(&path).spawn();
                            }
                        }
                        Err(err) => {
                            self.compiler_download_state = CompilerDownloadState::Failed(err);
                        }
                    }
                }
            }
        }
    }

    fn validate_api_key(provider: &str, key: &str) -> Result<(), &'static str> {
        let key = key.trim();
        if key.is_empty() {
            return Err("API key cannot be empty");
        }
        match provider {
            "OpenAI" => {
                if !key.starts_with("sk-") {
                    return Err("OpenAI API key must start with 'sk-'");
                }
            }
            "Anthropic" => {
                if !key.starts_with("sk-ant-") {
                    return Err("Anthropic API key must start with 'sk-ant-'");
                }
            }
            "Gemini" => {
                if !key.starts_with("AIza") {
                    return Err("Gemini API key must start with 'AIza'");
                }
            }
            "Mistral" | "Cohere" | "Custom" => {
                if key.len() < 8 {
                    return Err("API key seems too short");
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn llm_settings_dialog(&mut self, ctx: &egui::Context) {
        let mut open = self.show_llm_settings;
        let mut close_requested = false;
        egui::Window::new("llm_settings_dialog")
            .title_bar(false)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .frame(
                egui::Frame::window(&ctx.global_style())
                    .inner_margin(egui::Margin { left: 24, right: 24, top: 16, bottom: 24 })
                    .corner_radius(8),
            )
            .show(ctx, |ui| {
                ui.style_mut().text_styles.insert(egui::TextStyle::Body, egui::FontId::proportional(16.0));
                ui.style_mut().text_styles.insert(egui::TextStyle::Button, egui::FontId::proportional(16.0));
                ui.spacing_mut().button_padding = egui::vec2(16.0, 8.0);
                
                ui.horizontal(|ui| {
                    ui.heading(egui::RichText::new("Integrate LLM").strong().size(20.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(crate::components::button::borderless("✖")).clicked() {
                            close_requested = true;
                        }
                    });
                });
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(16.0);

                egui::Grid::new("llm_settings_grid")
                    .num_columns(2)
                    .spacing([24.0, 24.0])
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.add_space(8.0);
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                                ui.label("Provider:");
                            });
                        });
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            let old_provider = self.llm_provider.clone();
                            crate::components::dropdown::dropdown("llm_provider_combo", &self.llm_provider)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.llm_provider, "OpenAI".to_string(), "OpenAI");
                                    ui.selectable_value(&mut self.llm_provider, "Anthropic".to_string(), "Anthropic");
                                    ui.selectable_value(&mut self.llm_provider, "Gemini".to_string(), "Gemini");
                                    ui.selectable_value(&mut self.llm_provider, "Mistral".to_string(), "Mistral");
                                    ui.selectable_value(&mut self.llm_provider, "Cohere".to_string(), "Cohere");
                                    ui.selectable_value(&mut self.llm_provider, "Custom".to_string(), "Custom");
                                });
                            if old_provider != self.llm_provider {
                                self.llm_api_key_error = None;
                            }
                        });
                        ui.end_row();

                        ui.vertical(|ui| {
                            ui.add_space(10.0);
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                                ui.label("API Key:");
                            });
                        });
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            if self.has_saved_llm_key {
                                ui.add_enabled(false, crate::components::textfield::password(&mut self.llm_api_key.clone())
                                    .margin(egui::vec2(12.0, 10.0))
                                    .desired_width(240.0));
                                ui.add_space(8.0);
                                if ui.button("Remove").clicked() {
                                    let dialog = rfd::MessageDialog::new()
                                        .set_title("Remove API Key")
                                        .set_description("Are you sure you want to remove the API key ?")
                                        .set_buttons(rfd::MessageButtons::YesNo);
                                    if dialog.show() == rfd::MessageDialogResult::Yes {
                                        self.llm_api_key.clear();
                                        self.has_saved_llm_key = false;
                                        Self::remove_llm_api_key();
                                    }
                                }
                            } else {
                                ui.add(crate::components::textfield::password(&mut self.llm_api_key)
                                    .margin(egui::vec2(12.0, 10.0))
                                    .desired_width(320.0));
                            }
                        });
                        ui.end_row();
                    });

                ui.add_space(16.0);
                if let Some(err) = &self.llm_api_key_error {
                    ui.label(egui::RichText::new(err).color(egui::Color32::RED));
                    ui.add_space(8.0);
                }
                ui.separator();
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.has_saved_llm_key {
                            if ui.button("Close").clicked() {
                                self.llm_api_key_error = None;
                                close_requested = true;
                            }
                        } else {
                            if ui.button("Add").clicked() {
                                match Self::validate_api_key(&self.llm_provider, &self.llm_api_key) {
                                    Ok(_) => {
                                        Self::save_llm_api_key(&self.llm_api_key);
                                        self.has_saved_llm_key = true;
                                        self.llm_api_key_error = None;
                                        close_requested = true;
                                    }
                                    Err(e) => {
                                        self.llm_api_key_error = Some(e.to_string());
                                    }
                                }
                            }
                            ui.add_space(16.0);
                            if ui.button("Cancel").clicked() {
                                self.llm_api_key_error = None;
                                close_requested = true;
                            }
                        }
                    });
                });
            });
            
        if close_requested {
            self.show_llm_settings = false;
        } else {
            self.show_llm_settings = open;
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
                                    crate::components::button::standard(egui::RichText::new("×").size(13.0))
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
        let log = match self.output_panel_tab {
            OutputPanelTab::Compiler => self.active_tab().output_log.clone(),
            OutputPanelTab::AI => self.active_tab().ai_output_log.clone(),
        };
        let text_color = ui.style().visuals.text_color();
        egui::Frame::NONE
            .fill(if ui.visuals().dark_mode {
                egui::Color32::from_rgb(30, 30, 35)
            } else {
                egui::Color32::from_rgb(245, 245, 248)
            })
            .inner_margin(8.0)
            .corner_radius(egui::CornerRadius::same(6))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.scope(|ui| {
                        let is_dark = ui.visuals().dark_mode;
                        ui.style_mut().spacing.button_padding = egui::vec2(16.0, 8.0); // Bigger tabs
                        ui.style_mut().visuals.widgets.active.corner_radius = egui::CornerRadius::same(6);
                        ui.style_mut().visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(6);
                        ui.style_mut().visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(6);
                        ui.style_mut().visuals.selection.bg_fill = if is_dark {
                            egui::Color32::from_rgb(65, 65, 75) // Darker background
                        } else {
                            egui::Color32::from_rgb(215, 215, 225)
                        };
                        
                        let compiler_text = egui::RichText::new("Compiler Output").size(14.0).strong();
                        ui.selectable_value(&mut self.output_panel_tab, OutputPanelTab::Compiler, compiler_text);
                        
                        if self.has_saved_llm_key {
                            let ai_text = egui::RichText::new("AI Output").size(14.0).strong();
                            ui.selectable_value(&mut self.output_panel_tab, OutputPanelTab::AI, ai_text);
                        }
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.scope(|ui| {
                            ui.style_mut().spacing.button_padding = egui::vec2(16.0, 8.0);
                            ui.style_mut().visuals.widgets.active.corner_radius = egui::CornerRadius::same(6);
                            ui.style_mut().visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(6);
                            ui.style_mut().visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(6);
                            
                            if ui.button(egui::RichText::new("Clear").size(14.0).strong()).clicked() {
                                if !self.tabs.is_empty() {
                                    match self.output_panel_tab {
                                        OutputPanelTab::Compiler => self.active_tab_mut().output_log.clear(),
                                        OutputPanelTab::AI => self.active_tab_mut().ai_output_log.clear(),
                                    }
                                }
                            }
                        });
                    });
                });
                ui.separator();
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        if log.is_empty() {
                            ui.colored_label(
                                egui::Color32::GRAY,
                                egui::RichText::new("No output to display.").italics(),
                            );
                        } else {
                            ui.spacing_mut().item_spacing.y = 4.0;
                            for (text, color) in &log {
                                let c = if *color == egui::Color32::WHITE {
                                    text_color
                                } else {
                                    *color
                                };
                                ui.label(egui::RichText::new(text).color(c).family(egui::FontFamily::Monospace).size(13.0));
                            }
                        }
                    });
            });
    }

    fn compiler_dialog_ui(&mut self, ctx: &egui::Context) {
        let mut open = self.show_compiler_dialog;
        let mut close_requested = false;
        
        egui::Window::new("compiler_dialog_ui")
            .title_bar(false)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .frame(
                egui::Frame::window(&ctx.global_style())
                    .inner_margin(egui::Margin { left: 24, right: 24, top: 16, bottom: 24 })
                    .corner_radius(8),
            )
            .show(ctx, |ui| {
                ui.style_mut().text_styles.insert(egui::TextStyle::Body, egui::FontId::proportional(16.0));
                ui.style_mut().text_styles.insert(egui::TextStyle::Button, egui::FontId::proportional(16.0));
                ui.spacing_mut().button_padding = egui::vec2(16.0, 8.0);

                ui.horizontal(|ui| {
                    ui.heading(egui::RichText::new("Missing LaTeX Compiler").strong().size(20.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(crate::components::button::borderless("✖")).clicked() {
                            close_requested = true;
                        }
                    });
                });
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(16.0);

                ui.vertical_centered(|ui| {
                    ui.label("The LaTeX compiler (pdflatex) was not found on your system.");
                    ui.add_space(8.0);
                    ui.label("Would you like to download and install the recommended TeX Live package (medium scheme)?");
                    ui.add_space(24.0);
                    
                    match &self.compiler_download_state {
                        CompilerDownloadState::None => {}
                        CompilerDownloadState::Downloading(progress) => {
                            ui.label("Downloading installer...");
                            ui.add_space(16.0);
                            
                            let rect = ui.available_rect_before_wrap();
                            let size = egui::vec2(rect.width(), 20.0);
                            let (_rect, _response) = ui.allocate_exact_size(size, egui::Sense::hover());
                            let corner_radius = egui::CornerRadius::same(4);
                            ui.painter().rect_filled(
                                _rect,
                                corner_radius,
                                ui.visuals().extreme_bg_color,
                            );
                            let fill_width = _rect.width() * *progress;
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
                                format!("{:.0}%", *progress * 100.0),
                                egui::FontId::proportional(14.0),
                                ui.visuals().text_color(),
                            );
                        }
                        CompilerDownloadState::Complete => {
                            ui.label(egui::RichText::new("Download complete! Launching installer...").color(egui::Color32::from_rgb(60, 180, 75)));
                        }
                        CompilerDownloadState::Failed(err) => {
                            ui.label(egui::RichText::new(format!("Download failed: {}", err)).color(egui::Color32::from_rgb(220, 60, 60)));
                        }
                    }
                });

                match &self.compiler_download_state {
                    CompilerDownloadState::Downloading(_) => {}
                    CompilerDownloadState::None => {
                        ui.add_space(16.0);
                        ui.separator();
                        ui.add_space(16.0);
                        ui.horizontal(|ui| {
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("Download").clicked() {
                                    self.start_compiler_download();
                                }
                                ui.add_space(16.0);
                                if ui.button("Cancel").clicked() {
                                    close_requested = true;
                                }
                            });
                        });
                    }
                    CompilerDownloadState::Complete => {
                        ui.add_space(16.0);
                        ui.separator();
                        ui.add_space(16.0);
                        ui.horizontal(|ui| {
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("Close").clicked() {
                                    close_requested = true;
                                }
                            });
                        });
                    }
                    CompilerDownloadState::Failed(_) => {
                        ui.add_space(24.0);
                        ui.separator();
                        ui.add_space(16.0);
                        ui.horizontal(|ui| {
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("Retry").clicked() {
                                    self.start_compiler_download();
                                }
                                ui.add_space(16.0);
                                if ui.button("Cancel").clicked() {
                                    close_requested = true;
                                }
                            });
                        });
                    }
                }
            });
            
        if close_requested || !open {
            self.show_compiler_dialog = false;
            if close_requested {
                self.compiler_download_state = CompilerDownloadState::None;
            }
        }
    }
    
    fn start_compiler_download(&mut self) {
        self.compiler_download_state = CompilerDownloadState::Downloading(0.0);
        let tx = self.update_tx.clone();
        
        std::thread::spawn(move || {
            #[cfg(target_os = "windows")]
            let url = "https://mirror.ctan.org/systems/windows/protext/protext-3.2-021024.zip";
            #[cfg(target_os = "linux")]
            let url = "https://mirror.ctan.org/systems/texlive/tlnet/install-tl-unx.tar.gz";
            #[cfg(target_os = "macos")]
            let url = "https://mirror.ctan.org/systems/mac/mactex/mactex-basic.pkg";
            
            match ureq::get(url).header("User-Agent", "lekhani-latex").call() {
                Ok(response) => {
                    let len: Option<u64> = response
                        .headers()
                        .get("Content-Length")
                        .and_then(|h| h.to_str().ok())
                        .and_then(|s| s.parse().ok());
                        
                    let mut reader = response.into_body().into_reader();
                    let download_dir = dirs::download_dir().unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
                    
                    #[cfg(target_os = "windows")]
                    let file_name = "protext.zip";
                    #[cfg(target_os = "linux")]
                    let file_name = "install-tl-unx.tar.gz";
                    #[cfg(target_os = "macos")]
                    let file_name = "mactex-basic.pkg";
                    
                    let out_path = download_dir.join(file_name);
                    
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
                                        let progress = (downloaded as f32) / (total as f32);
                                        let _ = tx.send(UpdateMessage::CompilerDownloadProgress(progress));
                                    }
                                }
                                Err(e) => {
                                    let _ = tx.send(UpdateMessage::CompilerDownloadComplete(Err(format!("Network error: {}", e))));
                                    return;
                                }
                            }
                        }
                        let _ = tx.send(UpdateMessage::CompilerDownloadComplete(Ok(out_path)));
                    } else {
                        let _ = tx.send(UpdateMessage::CompilerDownloadComplete(Err("Failed to save file. Check disk permissions.".into())));
                    }
                }
                Err(e) => {
                    let _ = tx.send(UpdateMessage::CompilerDownloadComplete(Err(format!("Connection failed: {}", e))));
                }
            }
        });
    }
}
