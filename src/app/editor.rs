use egui::{Color32, FontId, ScrollArea, TextEdit};
use crate::app::App;
use crate::completions;
use crate::lexer;
use crate::types::Theme;

impl App {
    pub(super) fn editor_area(&mut self, ui: &mut egui::Ui) {
        if self.tabs.is_empty() {
            return;
        }
        let line_count = self.active_tab().buffer.line_count();
        let gutter_width = if line_count > 0 {
            let digits = line_count.to_string().len();
            (digits as f32 * 9.0 + 16.0).max(36.0)
        } else {
            36.0
        };

        ui.horizontal_top(|ui| {
            let height = ui.available_height();
            let (gutter_rect, _) = ui.allocate_exact_size(
                egui::Vec2::new(gutter_width, height),
                egui::Sense::hover(),
            );
            self.paint_gutter(ui, gutter_rect, line_count);

            self.text_edit_area(ui);
        });
    }

    fn paint_gutter(&self, ui: &egui::Ui, rect: egui::Rect, line_count: usize) {
        let painter = ui.painter_at(rect);
        let ctx = ui.ctx();
        let bg = self.theme.gutter_bg(ctx);
        painter.rect_filled(rect, 0.0, bg);

        let sep_x = rect.right() - 1.0;
        let sep_color = self.theme.gutter_sep(ctx);
        painter.line_segment(
            [
                egui::pos2(sep_x, rect.top()),
                egui::pos2(sep_x, rect.bottom()),
            ],
            egui::Stroke::new(1.0, sep_color),
        );

        let text_color = self.theme.gutter_text(ctx);
        let font_id = FontId::monospace(12.0);

        let line_height = 16.0;
        let start_y = rect.top() + 4.0;

        for i in 1..=line_count {
            let y = start_y + (i - 1) as f32 * line_height;
            if y > rect.bottom() {
                break;
            }
            painter.text(
                egui::pos2(rect.right() - 6.0, y),
                egui::Align2::RIGHT_TOP,
                &i.to_string(),
                font_id.clone(),
                text_color,
            );
        }
    }

    fn text_edit_area(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme;
        let ctx = ui.ctx().clone();

        let tab = self.active_tab_mut();
        let mut text = std::mem::take(&mut tab.buffer.text);
        let mut layouter =
            move |layouter_ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap_width: f32| {
                let text = buf.as_str();
                let tokens = lexer::tokenize(text);

                let syn = theme.syntax_colors(&ctx);

                let job = egui::text::LayoutJob {
                    text: text.into(),
                    sections: tokens
                        .iter()
                        .map(|token| {
                            let color = match token.token_type {
                                lexer::TokenType::Command => syn.cmd,
                                lexer::TokenType::MathDollar
                                | lexer::TokenType::MathDoubleDollar => syn.math,
                                lexer::TokenType::OpenBrace
                                | lexer::TokenType::CloseBrace => syn.brace,
                                lexer::TokenType::Comment => syn.comment,
                                lexer::TokenType::Text => syn.text,
                            };
                            egui::text::LayoutSection {
                                leading_space: 0.0,
                                byte_range: token.start..token.end,
                                format: egui::text::TextFormat {
                                    font_id: FontId::monospace(14.0),
                                    color,
                                    ..Default::default()
                                },
                            }
                        })
                        .collect(),
                    wrap: egui::text::TextWrapping {
                        max_width: wrap_width,
                        ..Default::default()
                    },
                    ..Default::default()
                };

                layouter_ui.fonts_mut(|f| f.layout_job(job))
            };

        let response = ui.add_sized(
            ui.available_size(),
            TextEdit::multiline(&mut text)
                .code_editor()
                .desired_width(f32::INFINITY)
                .layouter(&mut layouter),
        );

        let cursor_char = egui::TextEdit::load_state(ui.ctx(), response.id)
            .and_then(|state| state.cursor.char_range())
            .map_or(0, |range| range.primary.index);
        let cursor_pos = text
            .char_indices()
            .nth(cursor_char)
            .map_or(text.len(), |(b, _)| b);
        let changed = response.changed();
        let tab = self.active_tab_mut();
        tab.buffer.text = text;
        tab.buffer.cursor = cursor_pos;
        if changed {
            tab.buffer.sync_after_edit();
        }

        self.completion_visible = false;
        if response.has_focus() && cursor_pos > 0 {
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
                        self.completion_visible = true;
                        self.completion_matches =
                            matches.into_iter().map(|s| s.to_string()).collect();
                        self.completion_byte_range = Some((bslash, cursor));
                    }
                }
            }
        }

        if self.completion_visible {
            let line_height = 16.0;
            let (cursor_line, _) = self.active_tab().buffer.cursor_line_col();
            let popup_pos = egui::pos2(
                response.rect.left() + 4.0,
                response.rect.top() + (cursor_line as f32) * line_height + line_height,
            );
            let matches = self.completion_matches.clone();
            let range = self.completion_byte_range;
            let resolved = self.theme.resolve(ui.ctx());
            let bg_fill = match resolved {
                Theme::Dark => Color32::from_rgb(40, 44, 52),
                Theme::Light => Color32::from_rgb(255, 255, 255),
                Theme::System => unreachable!(),
            };

            let mut close = false;
            let mut selected: Option<usize> = None;
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
                                    if ui.button(&label).clicked() {
                                        close = true;
                                        selected = Some(i);
                                    }
                                }
                            });
                    });
                });

            if close {
                if let Some(idx) = selected {
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
            }
        }
    }
}
