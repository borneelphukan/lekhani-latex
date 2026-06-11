use std::path::Path;

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
    pub preview_texture: Option<egui::TextureHandle>,
    pub status_message: String,
    pub error_message: Option<String>,
    pub scroll_offset: egui::Vec2,
}

impl Tab {
    fn new(title: String, buffer: EditorBuffer, status_message: String) -> Self {
        Self {
            title,
            buffer,
            compiler: CompilerBridge::new(CompilerConfig::default()),
            preview: PreviewViewer::new(),
            show_preview: true,
            preview_texture: None,
            status_message,
            error_message: None,
            scroll_offset: egui::Vec2::ZERO,
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
