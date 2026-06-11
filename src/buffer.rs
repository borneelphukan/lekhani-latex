#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use crate::types::{AppError, EditKind, EditToken};

const MAX_UNDO: usize = 1024;

#[derive(Debug)]
pub struct EditorBuffer {
    pub text: String,
    path: Option<PathBuf>,
    pub dirty: bool,
    pub cursor: usize,
    undo_stack: Vec<EditToken>,
    redo_stack: Vec<EditToken>,
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

    pub fn save(&mut self) -> Result<(), AppError> {
        match &self.path {
            Some(path) => {
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }
                let tmp = path.with_extension("tex.tmp");
                fs::write(&tmp, &self.text)?;
                let _ = fs::remove_file(path);
                fs::rename(&tmp, path)?;
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
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("tex.tmp");
        fs::write(&tmp, &self.text)?;
        let _ = fs::remove_file(path);
        fs::rename(&tmp, path)?;
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

    pub fn insert_str(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        self.push_undo(EditToken {
            kind: EditKind::Insert(s.to_string()),
            position: self.cursor,
        });
        self.text.insert_str(self.cursor, s);
        self.cursor += s.len();
        self.dirty = true;
        self.rebuild_line_starts();
    }

    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let prev = self.text[..self.cursor].chars().next_back().unwrap();
        let len = prev.len_utf8();
        let deleted = self.text[self.cursor - len..self.cursor].to_string();
        self.push_undo(EditToken {
            kind: EditKind::Delete(deleted),
            position: self.cursor - len,
        });
        self.text.drain(self.cursor - len..self.cursor);
        self.cursor -= len;
        self.dirty = true;
        self.rebuild_line_starts();
    }

    pub fn delete(&mut self) {
        if self.cursor >= self.text.len() {
            return;
        }
        let next = self.text[self.cursor..].chars().next().unwrap();
        let len = next.len_utf8();
        let deleted = self.text[self.cursor..self.cursor + len].to_string();
        self.push_undo(EditToken {
            kind: EditKind::Delete(deleted),
            position: self.cursor,
        });
        self.text.drain(self.cursor..self.cursor + len);
        self.dirty = true;
        self.rebuild_line_starts();
    }

    pub fn move_left(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let c = self.text[..self.cursor].chars().next_back().unwrap();
        self.cursor -= c.len_utf8();
    }

    pub fn move_right(&mut self) {
        if self.cursor >= self.text.len() {
            return;
        }
        let c = self.text[self.cursor..].chars().next().unwrap();
        self.cursor += c.len_utf8();
    }

    pub fn move_up(&mut self) {
        let (line, col) = self.cursor_line_col();
        if line <= 1 {
            self.cursor = 0;
            return;
        }
        let prev_line_start = self.line_starts[line - 2];
        let prev_line_end = self.line_starts[line - 1];
        let prev_line = &self.text[prev_line_start..prev_line_end];
        let prev_line_len = prev_line.chars().count();
        let target_col = col.min(prev_line_len.saturating_sub(1));
        let byte_idx = prev_line
            .char_indices()
            .nth(target_col)
            .map(|(i, _)| i)
            .unwrap_or(prev_line.len());
        self.cursor = prev_line_start + byte_idx;
    }

    pub fn move_down(&mut self) {
        let (line, col) = self.cursor_line_col();
        if line >= self.line_count() {
            self.cursor = self.text.len();
            return;
        }
        let next_line_start = self.line_starts[line];
        let next_line_end = if line + 1 < self.line_starts.len() {
            self.line_starts[line + 1]
        } else {
            self.text.len()
        };
        let next_line = &self.text[next_line_start..next_line_end];
        let next_line_len = next_line.chars().count();
        let target_col = col.min(next_line_len.saturating_sub(1));
        let byte_idx = next_line
            .char_indices()
            .nth(target_col)
            .map(|(i, _)| i)
            .unwrap_or(next_line.len());
        self.cursor = next_line_start + byte_idx;
    }

    pub fn move_home(&mut self) {
        let line = self.cursor_line_col().0;
        if line <= self.line_starts.len() {
            self.cursor = self.line_starts[line - 1];
        }
    }

    pub fn move_end(&mut self) {
        let line = self.cursor_line_col().0;
        if line <= self.line_starts.len() {
            let end = if line < self.line_starts.len() {
                self.line_starts[line].saturating_sub(1)
            } else {
                self.text.len()
            };
            self.cursor = end;
        }
    }

    pub fn undo(&mut self) {
        if let Some(token) = self.undo_stack.pop() {
            self.cursor = token.position;
            match &token.kind {
                EditKind::Insert(text) => {
                    self.text.drain(self.cursor..self.cursor + text.len());
                }
                EditKind::Delete(text) => {
                    self.text.insert_str(self.cursor, text);
                    self.cursor += text.len();
                }
            }
            self.redo_stack.push(token);
            self.dirty = true;
            self.rebuild_line_starts();
        }
    }

    pub fn redo(&mut self) {
        if let Some(token) = self.redo_stack.pop() {
            match &token.kind {
                EditKind::Insert(text) => {
                    self.cursor = token.position;
                    self.text.insert_str(self.cursor, text);
                    self.cursor += text.len();
                }
                EditKind::Delete(text) => {
                    self.cursor = token.position;
                    self.text.drain(self.cursor..self.cursor + text.len());
                }
            }
            self.undo_stack.push(token);
            self.dirty = true;
            self.rebuild_line_starts();
        }
    }

    fn push_undo(&mut self, token: EditToken) {
        self.undo_stack.push(token);
        if self.undo_stack.len() > MAX_UNDO {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    fn rebuild_line_starts(&mut self) {
        self.line_starts.clear();
        self.line_starts.push(0);
        for (i, c) in self.text.char_indices() {
            if c == '\n' {
                self.line_starts.push(i + 1);
            }
        }
    }

    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    pub fn cursor_line_col(&self) -> (usize, usize) {
        if self.line_starts.is_empty() {
            return (1, 0);
        }
        let line = match self.line_starts.binary_search(&self.cursor) {
            Ok(idx) => idx + 1,
            Err(idx) => idx,
        };
        let line_idx = line.saturating_sub(1);
        if line_idx >= self.line_starts.len() {
            return (self.line_starts.len(), 0);
        }
        let line_start = self.line_starts[line_idx];
        let col = self.text[line_start..self.cursor].chars().count();
        (line, col)
    }

    pub fn line_start(&self, line: usize) -> Option<usize> {
        if line == 0 || line > self.line_starts.len() {
            return None;
        }
        Some(self.line_starts[line - 1])
    }

    pub fn line_text(&self, line: usize) -> Option<&str> {
        let start = self.line_start(line)?;
        let end = if line < self.line_starts.len() {
            self.line_starts[line]
        } else {
            self.text.len()
        };
        Some(&self.text[start..end])
    }

    pub fn indentation_at_cursor(&self) -> String {
        let (line, _) = self.cursor_line_col();
        if let Some(text) = self.line_text(line) {
            let indent: String = text.chars().take_while(|c| *c == ' ' || *c == '\t').collect();
            indent
        } else {
            String::new()
        }
    }
}
