use crate::buffer::EditorBuffer;
use crate::types::{EditKind, EditToken};

impl EditorBuffer {
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

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub(crate) fn push_undo(&mut self, token: EditToken) {
        self.undo_stack.push(token);
        if self.undo_stack.len() > super::MAX_UNDO {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    pub fn replace_all(&mut self, new_text: &str) {
        let old_text = self.text.clone();
        if old_text == new_text {
            return;
        }
        // First delete everything
        self.push_undo(EditToken {
            kind: EditKind::Delete(old_text),
            position: 0,
        });
        self.text.clear();
        self.cursor = 0;
        
        // Then insert new text
        self.push_undo(EditToken {
            kind: EditKind::Insert(new_text.to_string()),
            position: 0,
        });
        self.text = new_text.to_string();
        self.cursor = new_text.len();
        self.dirty = true;
        self.rebuild_line_starts();
    }
}
