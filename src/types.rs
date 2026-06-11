use std::fmt;

#[allow(dead_code)]
#[derive(Debug)]
pub enum AppError {
    Io(std::io::Error),
    Compile(String),
    Preview(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Io(e) => write!(f, "I/O error: {}", e),
            AppError::Compile(e) => write!(f, "Compile error: {}", e),
            AppError::Preview(e) => write!(f, "Preview error: {}", e),
        }
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct CursorPos {
    pub line: usize,
    pub col: usize,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum EditKind {
    Insert(String),
    Delete(String),
}

#[derive(Debug, Clone)]
pub struct EditToken {
    pub kind: EditKind,
    pub position: usize,
}

use egui::Color32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
}

pub struct SyntaxColors {
    pub text: Color32,
    pub cmd: Color32,
    pub math: Color32,
    pub brace: Color32,
    pub comment: Color32,
}

impl Theme {
    pub fn active_tab_bg(self) -> Color32 {
        match self {
            Theme::Dark => Color32::from_rgb(40, 44, 52),
            Theme::Light => Color32::from_rgb(240, 240, 244),
        }
    }

    pub fn gutter_bg(self) -> Color32 {
        match self {
            Theme::Dark => Color32::from_rgb(30, 34, 40),
            Theme::Light => Color32::from_rgb(240, 240, 244),
        }
    }

    pub fn gutter_sep(self) -> Color32 {
        match self {
            Theme::Dark => Color32::from_rgb(48, 54, 62),
            Theme::Light => Color32::from_rgb(210, 210, 215),
        }
    }

    pub fn gutter_text(self) -> Color32 {
        match self {
            Theme::Dark => Color32::from_rgb(100, 110, 130),
            Theme::Light => Color32::from_rgb(150, 155, 165),
        }
    }

    pub fn syntax_colors(self) -> SyntaxColors {
        match self {
            Theme::Dark => SyntaxColors {
                text: Color32::from_rgb(220, 220, 224),
                cmd: Color32::from_rgb(86, 156, 214),
                math: Color32::from_rgb(214, 157, 133),
                brace: Color32::from_rgb(255, 215, 0),
                comment: Color32::from_rgb(106, 153, 85),
            },
            Theme::Light => SyntaxColors {
                text: Color32::from_rgb(30, 30, 34),
                cmd: Color32::from_rgb(0, 56, 168),
                math: Color32::from_rgb(196, 86, 4),
                brace: Color32::from_rgb(180, 120, 0),
                comment: Color32::from_rgb(0, 128, 0),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompilerConfig {
    pub command: String,
    pub args: Vec<String>,
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            command: "pdflatex".into(),
            args: vec![
                "-interaction=nonstopmode".into(),
                "-halt-on-error".into(),
            ],
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompileStatus {
    Idle,
    Running,
    Success,
    Failed,
}
