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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
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
