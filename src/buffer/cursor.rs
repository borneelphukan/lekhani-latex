use crate::buffer::EditorBuffer;

impl EditorBuffer {
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
