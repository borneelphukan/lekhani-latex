use std::path::Path;
use std::time::Instant;

use egui::Color32;

use crate::buffer::EditorBuffer;
use crate::compiler::CompilerBridge;
use crate::preview::PreviewViewer;
use crate::types::CompilerConfig;

pub struct Tab {
    pub title: String,
    pub buffer: EditorBuffer,
    pub compiler: CompilerBridge,
    pub preview: PreviewViewer,
    pub show_preview: bool,
    pub preview_textures: std::collections::HashMap<usize, egui::TextureHandle>,
    pub status_message: String,
    pub error_message: Option<String>,
    pub error_lines: Vec<usize>,
    pub output_log: Vec<(String, Color32)>,
    pub ai_output_log: Vec<(String, Color32)>,
    pub scroll_offset: egui::Vec2,
    pub scroll_request: Option<egui::Vec2>,
    pub compile_start_time: Option<Instant>,
}

impl Tab {
    fn new(title: String, buffer: EditorBuffer, status_message: String) -> Self {
        Self {
            title,
            buffer,
            compiler: CompilerBridge::new(CompilerConfig::default()),
            preview: PreviewViewer::new(),
            show_preview: true,
            preview_textures: std::collections::HashMap::new(),
            status_message,
            error_message: None,
            error_lines: Vec::new(),
            output_log: Vec::new(),
            ai_output_log: Vec::new(),
            scroll_offset: egui::Vec2::ZERO,
            scroll_request: None,
            compile_start_time: None,
        }
    }

    pub fn load(path: &Path) -> Self {
        let title = Self::title_from(path);
        let buffer = EditorBuffer::load(path).unwrap_or_else(|_| {
            let mut b = EditorBuffer::new();
            b.set_path(Some(path.to_path_buf()));
            b
        });
        Self::new(title, buffer, format!("Opened {}", path.display()))
    }

    pub fn new_empty(path: &Path) -> Self {
        let title = Self::title_from(path);
        let mut buffer = EditorBuffer::new();
        buffer.set_path(Some(path.to_path_buf()));
        buffer.dirty = false;
        Self::new(title, buffer, format!("Created {}", path.display()))
    }

    fn title_from(path: &Path) -> String {
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled")
            .to_string()
    }
}
