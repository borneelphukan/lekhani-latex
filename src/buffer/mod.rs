#![allow(dead_code)]

pub mod cursor;
pub mod edit;

use std::fs;
use std::path::{Path, PathBuf};

use crate::types::AppError;

const MAX_UNDO: usize = 1024;

#[derive(Debug)]
pub struct EditorBuffer {
    pub text: String,
    path: Option<PathBuf>,
    pub dirty: bool,
    pub cursor: usize,
    undo_stack: Vec<crate::types::EditToken>,
    redo_stack: Vec<crate::types::EditToken>,
    line_starts: Vec<usize>,
}

impl EditorBuffer {
    pub fn new() -> Self {
        let text = concat!(
            "\\documentclass{article}\n",
            "\n",
            "% Preamble (Packages and Settings)\n",
            "\\usepackage[utf8]{inputenc} % Ensures correct character encoding\n",
            "\n",
            "\\title{Document Title}\n",
            "\\author{Author Name}\n",
            "\\date{\\today}\n",
            "\n",
            "\\begin{document}\n",
            "\n",
            "\\maketitle\n",
            "\n",
            "\\section{Introduction}\n",
            "Your text goes here.\n",
            "\n",
            "\\end{document}\n",
        )
        .to_string();
        let mut buf = Self {
            text,
            path: None,
            dirty: false,
            cursor: 0,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            line_starts: Vec::new(),
        };
        buf.rebuild_line_starts();
        buf
    }

    pub fn load(path: &Path) -> Result<Self, AppError> {
        let text = fs::read_to_string(path)?;
        let mut buf = Self::new();
        buf.text = text;
        buf.path = Some(path.to_path_buf());
        buf.dirty = false;
        buf.rebuild_line_starts();
        Ok(buf)
    }

    fn write_atomically(text: &str, path: &Path) -> Result<(), AppError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("tex.tmp");
        fs::write(&tmp, text)?;
        let _ = fs::remove_file(path);
        fs::rename(&tmp, path)?;
        Ok(())
    }

    pub fn save(&mut self) -> Result<(), AppError> {
        match &self.path {
            Some(path) => {
                Self::write_atomically(&self.text, path)?;
                self.dirty = false;
                Ok(())
            }
            None => Err(AppError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no file path set",
            ))),
        }
    }

    pub fn save_as(&mut self, path: &Path) -> Result<(), AppError> {
        Self::write_atomically(&self.text, path)?;
        self.path = Some(path.to_path_buf());
        self.dirty = false;
        Ok(())
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn set_path(&mut self, path: Option<PathBuf>) {
        self.path = path;
    }

    pub fn sync_after_edit(&mut self) {
        self.dirty = true;
        self.rebuild_line_starts();
    }

    pub(crate) fn rebuild_line_starts(&mut self) {
        self.line_starts.clear();
        self.line_starts.push(0);
        for (i, c) in self.text.char_indices() {
            if c == '\n' {
                self.line_starts.push(i + 1);
            }
        }
    }
}
