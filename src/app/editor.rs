use std::cell::Cell;
use std::time::Instant;
use egui::{Color32, ScrollArea, TextEdit};
use crate::app::App;
use crate::completions;
use crate::lexer;
use crate::types::Theme;

impl App {
    fn is_article_document(&self) -> bool {
        if self.tabs.is_empty() {
            return false;
        }
        let text = &self.active_tab().buffer.text;
        let re = regex::Regex::new(r"\\documentclass(?:\[.*?\])?\{(\w+)\}").unwrap();
        if let Some(caps) = re.captures(text) {
            caps.get(1).unwrap().as_str() == "article"
        } else {
            true
        }
    }

    fn detect_current_heading(&self) -> Option<&'static str> {
        if self.tabs.is_empty() {
            return None;
        }
        let tab = self.active_tab();
        let text = &tab.buffer.text;
        let cursor = tab.buffer.cursor.min(text.len());

        let line_start = text[..cursor].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let line_end = text[cursor..].find('\n').map(|i| i + cursor).unwrap_or(text.len());
        let line = text[line_start..line_end].trim();

        let re = regex::Regex::new(r"\\(part|chapter|section|subsection|subsubsection|paragraph|subparagraph)(\*?)\{").unwrap();
        re.captures(line).and_then(|caps| match caps.get(1).unwrap().as_str() {
            "part" => Some("part"),
            "chapter" => Some("chapter"),
            "section" => Some("section"),
            "subsection" => Some("subsection"),
            "subsubsection" => Some("subsubsection"),
            "paragraph" => Some("paragraph"),
            "subparagraph" => Some("subparagraph"),
            _ => None,
        })
    }

    fn display_heading_label(heading: Option<&str>) -> String {
        match heading {
            None => "Paragraph".to_string(),
            Some("subsection") => "Subsection".to_string(),
            Some("subsubsection") => "Sub-subsection".to_string(),
            Some("subparagraph") => "Subparagraph".to_string(),
            Some(s) => {
                let mut c = s.chars();
                c.next().unwrap().to_uppercase().to_string() + c.as_str()
            }
        }
    }

    fn apply_heading(&mut self, heading: Option<&str>, numbered: bool) {
        let tab = self.active_tab_mut();
        let text = &mut tab.buffer.text;
        let cursor = tab.buffer.cursor.min(text.len());

        let line_start = text[..cursor].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let line_end = text[cursor..].find('\n').map(|i| i + cursor).unwrap_or(text.len());
        let line = text[line_start..line_end].to_string();
        let trimmed = line.trim();

        let re = regex::Regex::new(r"\\(part|chapter|section|subsection|subsubsection|paragraph|subparagraph)(\*?)\{([^}]*)\}").unwrap();
        let content = if let Some(caps) = re.captures(trimmed) {
            caps.get(3).unwrap().as_str().trim().to_string()
        } else {
            trimmed.to_string()
        };

        let indent = &line[..line.len().saturating_sub(trimmed.len())];

        match heading {
            None => {
                let new_line = format!("{}{}", indent, content);
                text.replace_range(line_start..line_end, &new_line);
                tab.buffer.cursor = line_start + new_line.len();
            }
            Some(cmd) => {
                let star = if numbered { "" } else { "*" };
                let new_line = format!("{}\\{}{}{{{}}}", indent, cmd, star, content);
                text.replace_range(line_start..line_end, &new_line);
                tab.buffer.cursor = line_start + new_line.len() - 1;
            }
        }
        tab.buffer.sync_after_edit();
    }

    fn toggle_heading_starred(&mut self) {
        let want_star = !self.heading_numbered;
        let tab = self.active_tab_mut();
        let text = &mut tab.buffer.text;
        let cursor = tab.buffer.cursor.min(text.len());

        let line_start = text[..cursor].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let line_end = text[cursor..].find('\n').map(|i| i + cursor).unwrap_or(text.len());
        let old_len = line_end - line_start;
        let line = text[line_start..line_end].to_string();

        let re = regex::Regex::new(r"\\(part|chapter|section|subsection|subsubsection|paragraph|subparagraph)(\*?)\{").unwrap();
        if re.is_match(&line) {
            let new_line = if want_star {
                let add_re = regex::Regex::new(r"(\\(?:part|chapter|section|subsection|subsubsection|paragraph|subparagraph))(\{)").unwrap();
                add_re.replace(&line, "${1}*${2}").to_string()
            } else {
                let remove_re = regex::Regex::new(r"(\\(?:part|chapter|section|subsection|subsubsection|paragraph|subparagraph))\*(\{)").unwrap();
                remove_re.replace(&line, "${1}${2}").to_string()
            };
            if new_line != line {
                let delta = new_line.len() as isize - old_len as isize;
                text.replace_range(line_start..line_end, &new_line);
                if cursor > line_start {
                    let new_cursor = (cursor as isize + delta).max(line_start as isize) as usize;
                    tab.buffer.cursor = new_cursor.min(text.len());
                }
                tab.buffer.sync_after_edit();
            }
        }
    }

    pub(super) fn formatting_toolbar(&mut self, ui: &mut egui::Ui) {
        if self.tabs.is_empty() {
            return;
        }

        let is_article = self.is_article_document();
        let current_heading = self.detect_current_heading();
        let display_label = Self::display_heading_label(current_heading);

        ui.horizontal(|ui| {
            let btn_size = egui::vec2(28.0, 28.0);

            if ui.add_sized(btn_size, crate::components::button::icon(
                egui::RichText::new("B").size(14.0).strong(),
            )).on_hover_text("Bold").clicked() {
                let tab = self.active_tab_mut();
                tab.buffer.insert_str("\\textbf{}");
                tab.buffer.cursor -= 1;
            }
            if ui.add_sized(btn_size, crate::components::button::icon(
                egui::RichText::new("I").size(14.0).strong(),
            )).on_hover_text("Italic").clicked() {
                let tab = self.active_tab_mut();
                tab.buffer.insert_str("\\textit{}");
                tab.buffer.cursor -= 1;
            }
            if ui.add_sized(btn_size, crate::components::button::icon(
                egui::RichText::new("U").size(14.0).strong(),
            )).on_hover_text("Underline").clicked() {
                let tab = self.active_tab_mut();
                tab.buffer.insert_str("\\underline{}");
                tab.buffer.cursor -= 1;
            }

            ui.separator();

            crate::components::dropdown::dropdown("header_dropdown", &display_label)
                .width(140.0)
                .show_ui(ui, |ui| {
                    if ui.add(egui::Button::selectable(current_heading.is_none(), "Paragraph")).clicked() {
                        self.heading_numbered = true;
                        self.apply_heading(None, true);
                        ui.close();
                    }
                    ui.separator();

                    let items: [(&str, &str, f32); 7] = [
                        ("Part", "part", 16.0),
                        ("Chapter", "chapter", 15.0),
                        ("Section", "section", 14.0),
                        ("Subsection", "subsection", 13.5),
                        ("Sub-subsection", "subsubsection", 13.0),
                        ("Paragraph Header", "paragraph", 12.5),
                        ("Subparagraph", "subparagraph", 12.0),
                    ];

                    for &(label, cmd, size) in &items {
                        let is_selected = current_heading == Some(cmd);
                        let enabled = !(cmd == "chapter" && is_article);
                        let text = egui::RichText::new(label).size(size);
                        if ui.add_enabled(enabled, egui::Button::selectable(is_selected, text)).clicked() {
                            self.apply_heading(Some(cmd), self.heading_numbered);
                            ui.close();
                        }
                    }
                });

            if ui.checkbox(&mut self.heading_numbered, "Numbered").changed() {
                self.toggle_heading_starred();
            }

            ui.separator();
            let can_undo = self.active_tab().buffer.can_undo();
            if ui.add_enabled(can_undo, crate::components::button::standard("Undo"))
                .on_hover_text("Ctrl+Z")
                .clicked()
            {
                self.active_tab_mut().buffer.undo();
            }
            let can_redo = self.active_tab().buffer.can_redo();
            if ui.add_enabled(can_redo, crate::components::button::standard("Redo"))
                .on_hover_text("Ctrl+Y")
                .clicked()
            {
                self.active_tab_mut().buffer.redo();
            }
        });
    }

    pub(super) fn editor_area(&mut self, ui: &mut egui::Ui) {
        if self.tabs.is_empty() {
            return;
        }

        let error_lines = self.active_tab().error_lines.clone();

        let frame = egui::Frame {
            fill: ui.visuals().extreme_bg_color,
            inner_margin: egui::Margin::symmetric(4, 4),
            corner_radius: ui.visuals().widgets.noninteractive.corner_radius,
            stroke: ui.visuals().widgets.noninteractive.bg_stroke,
            ..Default::default()
        };

        frame.show(ui, |ui| {
            egui::ScrollArea::both()
                .id_salt("editor_main_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        self.text_edit_area(ui, &error_lines);
                    });
                });
        });
    }

    fn text_edit_area(&mut self, ui: &mut egui::Ui, error_lines: &[usize]) {
        let theme = self.theme;
        let ctx = ui.ctx().clone();
        let error_lines = error_lines.to_vec();

        let mut nav_up = false;
        let mut nav_down = false;
        let mut nav_enter = false;
        let mut nav_escape = false;

        if self.completion_visible {
            ctx.input_mut(|i| {
                let mut kept = Vec::new();
                for e in std::mem::take(&mut i.events) {
                    let consume = match &e {
                        egui::Event::Key {
                            key: egui::Key::ArrowDown,
                            pressed: true,
                            ..
                        } => {
                            nav_down = true;
                            true
                        }
                        egui::Event::Key {
                            key: egui::Key::ArrowUp,
                            pressed: true,
                            ..
                        } => {
                            nav_up = true;
                            true
                        }
                        egui::Event::Key {
                            key: egui::Key::Enter,
                            pressed: true,
                            ..
                        } => {
                            nav_enter = true;
                            true
                        }
                        egui::Event::Key {
                            key: egui::Key::Escape,
                            pressed: true,
                            ..
                        } => {
                            nav_escape = true;
                            true
                        }
                        _ => false,
                    };
                    if !consume {
                        kept.push(e);
                    }
                }
                i.events = kept;
            });
        }

        let tab = self.active_tab_mut();
        let mut text = std::mem::take(&mut tab.buffer.text);
        
        let id_source = "editor_text_edit";
        let edit_id = ui.make_persistent_id(id_source);
        let mut selection_byte_range = None;
        let popup_id = egui::Id::new("context_menu").with(edit_id);
        let menu_open = egui::Popup::is_id_open(ui.ctx(), popup_id);
        if menu_open {
            if let Some(state) = egui::TextEdit::load_state(ui.ctx(), edit_id) {
                if let Some(range) = state.cursor.char_range() {
                    let min_char = range.primary.index.min(range.secondary.index);
                    let max_char = range.primary.index.max(range.secondary.index);
                    if min_char < max_char {
                        let min_byte = text.char_indices().nth(min_char).map_or(text.len(), |(b, _)| b);
                        let max_byte = text.char_indices().nth(max_char).map_or(text.len(), |(b, _)| b);
                        selection_byte_range = Some(min_byte..max_byte);
                    }
                }
            }
        }
        let selection_bg = ctx.global_style().visuals.selection.bg_fill;

        let mut layouter =
            move |layouter_ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap_width: f32| {
                let text = buf.as_str();
                let tokens = lexer::tokenize(text);

                let syn = theme.syntax_colors(&ctx);

                let mut line_starts = vec![0usize];
                for (i, c) in text.char_indices() {
                    if c == '\n' {
                        line_starts.push(i + 1);
                    }
                }

                let line_of = |pos: usize| -> usize {
                    match line_starts.binary_search(&pos) {
                        Ok(i) => i + 1,
                        Err(i) => i,
                    }
                };

                let err_bg = if ctx.global_style().visuals.dark_mode {
                    Color32::from_rgb(90, 50, 55)
                } else {
                    Color32::from_rgb(255, 200, 200)
                };

                let mut sections = Vec::new();
                for token in tokens {
                    let color = match token.token_type {
                        lexer::TokenType::Command => syn.cmd,
                        lexer::TokenType::MathDollar
                        | lexer::TokenType::MathDoubleDollar => syn.math,
                        lexer::TokenType::OpenBrace
                        | lexer::TokenType::CloseBrace => syn.brace,
                        lexer::TokenType::Comment => syn.comment,
                        lexer::TokenType::Text => syn.text,
                    };
                    let line = line_of(token.start);
                    let base_bg = if error_lines.contains(&line) {
                        err_bg
                    } else {
                        Color32::TRANSPARENT
                    };
                    
                    if let Some(sel) = &selection_byte_range {
                        let overlap_start = token.start.max(sel.start);
                        let overlap_end = token.end.min(sel.end);
                        if overlap_start < overlap_end {
                            if token.start < overlap_start {
                                sections.push(egui::text::LayoutSection {
                                    leading_space: 0.0,
                                    byte_range: token.start..overlap_start,
                                    format: egui::text::TextFormat {
                                        font_id: egui::FontId::monospace(14.0),
                                        color,
                                        background: base_bg,
                                        ..Default::default()
                                    }
                                });
                            }
                            sections.push(egui::text::LayoutSection {
                                leading_space: 0.0,
                                byte_range: overlap_start..overlap_end,
                                format: egui::text::TextFormat {
                                    font_id: egui::FontId::monospace(14.0),
                                    color,
                                    background: selection_bg,
                                    ..Default::default()
                                }
                            });
                            if overlap_end < token.end {
                                sections.push(egui::text::LayoutSection {
                                    leading_space: 0.0,
                                    byte_range: overlap_end..token.end,
                                    format: egui::text::TextFormat {
                                        font_id: egui::FontId::monospace(14.0),
                                        color,
                                        background: base_bg,
                                        ..Default::default()
                                    }
                                });
                            }
                            continue;
                        }
                    }

                    sections.push(egui::text::LayoutSection {
                        leading_space: 0.0,
                        byte_range: token.start..token.end,
                        format: egui::text::TextFormat {
                            font_id: egui::FontId::monospace(14.0),
                            color,
                            background: base_bg,
                            ..Default::default()
                        },
                    });
                }

                let job = egui::text::LayoutJob {
                    text: text.into(),
                    sections,
                    wrap: egui::text::TextWrapping {
                        max_width: wrap_width,
                        ..Default::default()
                    },
                    ..Default::default()
                };

                layouter_ui.fonts_mut(|f| f.layout_job(job))
            };

        let id_source = "editor_text_edit";
        let edit_id = ui.make_persistent_id(id_source);
        
        let previous_state = egui::TextEdit::load_state(ui.ctx(), edit_id);
        
        let mut tab_pressed = false;
        if ui.memory(|mem| mem.has_focus(edit_id)) {
            ui.input_mut(|i| {
                if i.consume_key(egui::Modifiers::NONE, egui::Key::Tab) {
                    tab_pressed = true;
                }
            });
        }

        let output = TextEdit::multiline(&mut text)
            .id_source(id_source)
            .font(egui::TextStyle::Monospace)
            .frame(egui::Frame::NONE)
            .desired_width(f32::INFINITY)
            .lock_focus(true)
            .layouter(&mut layouter)
            .show(ui);
        
        let mut cursor_screen_pos = output.response.rect.left_bottom();
        if let Some(range) = &output.cursor_range {
            let cursor_rect = output.galley.pos_from_cursor(range.primary);
            cursor_screen_pos = output.galley_pos + cursor_rect.left_bottom().to_vec2();
        }
        
        let response = output.response;
        
        let secondary_down = ui.input(|i| i.pointer.button_down(egui::PointerButton::Secondary));
        let secondary_released = ui.input(|i| i.pointer.button_released(egui::PointerButton::Secondary));
        if secondary_down || secondary_released || response.secondary_clicked() {
            if let Some(state) = previous_state {
                state.store(ui.ctx(), response.id);
            }
        }
        
        let cursor_char = egui::TextEdit::load_state(ui.ctx(), response.id)
            .and_then(|state| state.cursor.char_range())
            .map_or(0, |range| range.primary.index);
        let mut cursor_pos = text
            .char_indices()
            .nth(cursor_char)
            .map_or(text.len(), |(b, _)| b);
        let mut changed = response.changed();
        
        if tab_pressed {
            text.insert_str(cursor_pos, "    ");
            changed = true;
            
            if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), response.id) {
                if let Some(mut range) = state.cursor.char_range() {
                    range.primary.index += 4;
                    range.secondary.index += 4;
                    state.cursor.set_char_range(Some(range));
                    state.store(ui.ctx(), response.id);
                }
            }
            cursor_pos += 4;
        }
        
        let mut do_cut = false;
        let mut do_copy = false;
        let mut do_paste = false;
        let mut do_select_all = false;
        
        response.context_menu(|ui| {
            if ui.button("Cut (Ctrl + X)").clicked() {
                do_cut = true;
                ui.memory_mut(|m| m.request_focus(response.id));
                ui.close();
            }
            if ui.button("Copy (Ctrl + C)").clicked() {
                do_copy = true;
                ui.memory_mut(|m| m.request_focus(response.id));
                ui.close();
            }
            if ui.button("Paste (Ctrl + V)").clicked() {
                do_paste = true;
                ui.memory_mut(|m| m.request_focus(response.id));
                ui.close();
            }
            if ui.button("Select All (Ctrl + A)").clicked() {
                do_select_all = true;
                ui.memory_mut(|m| m.request_focus(response.id));
                ui.close();
            }
        });

        if do_copy || do_cut {
            if let Some(state) = egui::TextEdit::load_state(ui.ctx(), response.id) {
                if let Some(range) = state.cursor.char_range() {
                    let min = range.primary.index.min(range.secondary.index);
                    let max = range.primary.index.max(range.secondary.index);
                    if min < max {
                        let selected_text: String = text.chars().skip(min).take(max - min).collect();
                        ui.ctx().copy_text(selected_text);
                        if do_cut {
                            let min_byte = text.char_indices().nth(min).map_or(text.len(), |(b, _)| b);
                            let max_byte = text.char_indices().nth(max).map_or(text.len(), |(b, _)| b);
                            text.replace_range(min_byte..max_byte, "");
                            changed = true;
                            
                            let mut new_state = state;
                            new_state.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(min))));
                            new_state.store(ui.ctx(), response.id);
                            cursor_pos = min_byte;
                        }
                    }
                }
            }
        }

        if do_paste {
            let mut clipboard = String::new();
            if let Ok(mut cb) = arboard::Clipboard::new() {
                if let Ok(clip_text) = cb.get_text() {
                    clipboard = clip_text;
                }
            }
            if !clipboard.is_empty() {
                if let Some(state) = egui::TextEdit::load_state(ui.ctx(), response.id) {
                    let mut min_char = cursor_char;
                    let mut max_char = cursor_char;
                    if let Some(range) = state.cursor.char_range() {
                        min_char = range.primary.index.min(range.secondary.index);
                        max_char = range.primary.index.max(range.secondary.index);
                    }
                    let min_byte = text.char_indices().nth(min_char).map_or(text.len(), |(b, _)| b);
                    let max_byte = text.char_indices().nth(max_char).map_or(text.len(), |(b, _)| b);
                    
                    text.replace_range(min_byte..max_byte, &clipboard);
                    changed = true;
                    
                    let pasted_chars = clipboard.chars().count();
                    let new_cursor_char = min_char + pasted_chars;
                    
                    let mut new_state = state;
                    new_state.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(new_cursor_char))));
                    new_state.store(ui.ctx(), response.id);
                    cursor_pos = min_byte + clipboard.len();
                }
            }
        }

        if do_select_all {
            if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), response.id) {
                let char_count = text.chars().count();
                state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                    egui::text::CCursor::new(0),
                    egui::text::CCursor::new(char_count)
                )));
                state.store(ui.ctx(), response.id);
            }
        }
        
        if changed {
            self.last_content_change = Some(Instant::now());
        }
        let tab = self.active_tab_mut();
        tab.buffer.text = text;
        tab.buffer.cursor = cursor_pos;
        if changed {
            tab.buffer.sync_after_edit();
        }

        self.completion_visible = false;
        if response.has_focus() && cursor_pos > 0 && (!self.completion_block_trigger || changed) {
            self.completion_block_trigger = false;
            let text = &self.active_tab().buffer.text;
            let cursor = cursor_pos.min(text.len());
            let before = &text[..cursor];

            if let Some(bslash) = before.rfind('\\') {
                let partial = &before[bslash..];
                if partial.len() > 1
                    && partial[1..].chars().all(|c| c.is_alphanumeric())
                {
                    let matches = completions::find_completions(partial);
                    if !matches.is_empty() {
                        let new_matches: Vec<String> =
                            matches.into_iter().map(|s| s.to_string()).collect();
                        let prefix = partial.to_string();
                        let prefix_changed = prefix != self.completion_prefix;
                        self.completion_visible = true;
                        self.completion_matches = new_matches;
                        self.completion_byte_range = Some((bslash, cursor));
                        self.completion_prefix = prefix;
                        if prefix_changed {
                            self.completion_selected = 0;
                        } else {
                            self.completion_selected = self
                                .completion_selected
                                .min(self.completion_matches.len() - 1);
                        }
                    }
                }
            }
        }

        if nav_escape {
            self.completion_visible = false;
            self.completion_prefix.clear();
        }
        if nav_up {
            if self.completion_selected == 0 {
                self.completion_selected = self.completion_matches.len() - 1;
            } else {
                self.completion_selected -= 1;
            }
        }
        if nav_down {
            self.completion_selected =
                (self.completion_selected + 1) % self.completion_matches.len();
        }

        if self.completion_visible {
            let popup_pos = cursor_screen_pos;
            let matches = self.completion_matches.clone();
            let range = self.completion_byte_range;
            let selected_index = self.completion_selected;
            let resolved = self.theme.resolve(ui.ctx());
            let bg_fill = match resolved {
                Theme::Dark => Color32::from_rgb(40, 44, 52),
                Theme::Light => Color32::from_rgb(255, 255, 255),
                Theme::System => unreachable!(),
            };

            let close = Cell::new(nav_enter);
            let selected: Cell<Option<usize>> =
                Cell::new(if nav_enter { Some(selected_index) } else { None });
            let popup_id = egui::Id::new("latex_completions");

            let _ = egui::Area::new(popup_id)
                .fixed_pos(popup_pos)
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    let mut style = (*ui.ctx().global_style()).clone();
                    style.visuals.widgets.noninteractive.bg_fill = bg_fill;
                    ui.set_style(style);

                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.set_min_width(180.0);
                        ui.set_max_height(300.0);
                        ScrollArea::vertical()
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                for (i, cmd) in matches.iter().enumerate() {
                                    let label = cmd.replacen('\\', "", 1);
                                    let is_selected = i == selected_index;
                                    let mut button = crate::components::button::standard(&label);
                                    if is_selected {
                                        let select_bg = match resolved {
                                            Theme::Dark => {
                                                Color32::from_rgb(60, 70, 90)
                                            }
                                            Theme::Light => {
                                                Color32::from_rgb(200, 200, 220)
                                            }
                                            Theme::System => unreachable!(),
                                        };
                                        button = button.fill(select_bg);
                                    }
                                    if ui.add(button).clicked() {
                                        close.set(true);
                                        selected.set(Some(i));
                                    }
                                }
                            });
                    });
                });

            if close.get() {
                if let Some(idx) = selected.get() {
                    if let Some(replacement) = matches.get(idx) {
                        if let Some((start, end)) = range {
                            let tab = self.active_tab_mut();
                            let text = &mut tab.buffer.text;
                            if start <= end && end <= text.len() {
                                text.replace_range(start..end, replacement);
                                tab.buffer.cursor = start + replacement.len();
                                tab.buffer.sync_after_edit();
                            }
                        }
                    }
                }
                self.completion_visible = false;
                self.completion_prefix.clear();
                self.completion_block_trigger = true;
            }
        }
    }
}
