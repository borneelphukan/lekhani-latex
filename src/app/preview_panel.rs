use egui::widgets::Image;
use egui::{Color32, ScrollArea};
use crate::app::App;

impl App {
    pub(super) fn preview_toolbar(&mut self, ui: &mut egui::Ui) {
        if self.tabs.is_empty() {
            return;
        }

        let zoom = self.active_tab().preview.zoom;
        let page = self.active_tab().preview.page;
        let num_pages = self.active_tab().preview.num_pages;

        let mut new_zoom = zoom;
        let mut new_page = page;

        if ui.button("−").clicked() {
            new_zoom = (zoom - 0.25).max(0.25);
        }
        ui.label(format!("{}%", (zoom * 100.0) as u32));
        if ui.button("+").clicked() {
            new_zoom = (zoom + 0.25).min(4.0);
        }
        ui.separator();
        let has_prev = page > 0;
        if ui.add_enabled(has_prev, egui::Button::new("\u{25C0}")).clicked() {
            new_page = page.saturating_sub(1);
        }
        ui.label(format!("Page {}", page + 1));
        let has_next = num_pages.map_or(true, |n| page + 1 < n);
        if ui.add_enabled(has_next, egui::Button::new("\u{25B6}")).clicked() {
            new_page = page + 1;
        }
        ui.separator();
        if ui.button(" \u{25C0} ").on_hover_text("Left").clicked() {
            self.active_tab_mut().scroll_offset.x -= 50.0;
        }
        if ui.button(" \u{25B6} ").on_hover_text("Right").clicked() {
            self.active_tab_mut().scroll_offset.x += 50.0;
        }
        if ui.button(" \u{25B2} ").on_hover_text("Up").clicked() {
            self.active_tab_mut().scroll_offset.y -= 50.0;
        }
        if ui.button(" \u{25BC} ").on_hover_text("Down").clicked() {
            self.active_tab_mut().scroll_offset.y += 50.0;
        }

        self.active_tab_mut().preview.zoom = new_zoom;
        if new_page != page {
            let pdf_path = self.active_tab().preview.last_pdf_path.clone();
            self.active_tab_mut().preview.page = new_page;
            if let Some(path) = pdf_path {
                self.active_tab_mut().preview.render_pdf(&path, new_page);
            }
        }
    }

    pub(super) fn preview_content(&mut self, ui: &mut egui::Ui) {
        if self.tabs.is_empty() {
            return;
        }

        let tex = self.active_tab().preview_texture.clone();
        let image_size = self.active_tab().preview.image_size;
        let mut zoom = self.active_tab().preview.zoom;
        let page = self.active_tab().preview.page;
        let mut scroll_offset = self.active_tab().scroll_offset;

        if tex.is_some() {
            let zoom_delta = ui.input(|i| i.zoom_delta());
            if zoom_delta != 1.0 {
                zoom = (zoom * zoom_delta).clamp(0.25, 4.0);
            }
        }

        let inner = ui.vertical(|ui| {
            if let Some(tex) = tex {
                let img_size = tex.size_vec2();

                if zoom == 1.0 && image_size.map_or(false, |s| s[0] > 0) {
                    let avail_width = ui.available_width() - 10.0;
                    if avail_width > 0.0 {
                        let fit = avail_width / img_size.x;
                        if fit < 1.0 {
                            zoom = fit.max(0.25).min(4.0);
                        }
                    }
                }

                let display_size = img_size * zoom;
                let image = Image::from_texture(
                    egui::load::SizedTexture::new(tex.id(), display_size),
                );

                let output = ScrollArea::both()
                    .scroll_source(egui::containers::scroll_area::ScrollSource::MOUSE_WHEEL)
                    .vertical_scroll_offset(scroll_offset.y)
                    .horizontal_scroll_offset(scroll_offset.x)
                    .show(ui, |ui| {
                        ui.set_min_size(display_size);
                        ui.add(image);
                    });

                scroll_offset = output.state.offset;
            } else {
                let avail = ui.available_size();
                let (_, resp) = ui.allocate_exact_size(avail, egui::Sense::hover());
                ui.painter().text(
                    resp.rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "No preview available",
                    egui::FontId::proportional(16.0),
                    Color32::GRAY,
                );
            }
        });
        let preview_rect = inner.response.rect;

        let (hover_pos, middle_down, delta) = ui.input(|i| {
            (i.pointer.hover_pos(), i.pointer.middle_down(), i.pointer.delta())
        });
        if let Some(pos) = hover_pos {
            if preview_rect.contains(pos) && middle_down {
                scroll_offset += delta;
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            }
        }

        self.active_tab_mut().scroll_offset = scroll_offset;
        self.active_tab_mut().preview.zoom = zoom;
        self.active_tab_mut().preview.page = page;
    }
}
