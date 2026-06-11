use egui::widgets::Image;
use egui::{Color32, ScrollArea};
use crate::app::App;

impl App {
    pub(super) fn preview_panel(&mut self, ui: &mut egui::Ui) {
        if self.tabs.is_empty() {
            return;
        }

        let show_error = self.active_tab().preview.render_error.is_some();
        let err_text = self.active_tab().preview.render_error.clone();
        let has_pdf_path = self.active_tab().preview.last_pdf_path.is_some();
        let zoom = self.active_tab().preview.zoom;
        let page = self.active_tab().preview.page;
        let tex = self.active_tab().preview_texture.clone();
        let num_pages = self.active_tab().preview.num_pages;
        let mut scroll_offset = self.active_tab().scroll_offset;

        let mut do_render = false;
        let mut new_zoom = zoom;
        let mut new_page = page;
        let mut open_externally = false;
        let mut pending_scroll: Option<egui::Vec2> = None;

        if tex.is_some() {
            let zoom_delta = ui.input(|i| i.zoom_delta());
            if zoom_delta != 1.0 {
                new_zoom = (zoom * zoom_delta).clamp(0.25, 4.0);
            }
        }

        let inner = ui.vertical(|ui| {
            ui.horizontal(|ui| {
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
                    new_page = page - 1;
                    do_render = true;
                }
                ui.label(format!("Page {}", page + 1));
                let has_next = num_pages.map_or(true, |n| page + 1 < n);
                if ui.add_enabled(has_next, egui::Button::new("\u{25B6}")).clicked() {
                    new_page = page + 1;
                    do_render = true;
                }
                ui.separator();
                if ui.button("\u{25C0} ").on_hover_text("Left").clicked() {
                    pending_scroll = Some(egui::vec2(-50.0, 0.0));
                }
                if ui.button("\u{25B6} ").on_hover_text("Right").clicked() {
                    pending_scroll = Some(egui::vec2(50.0, 0.0));
                }
                if ui.button("\u{25B2}").on_hover_text("Up").clicked() {
                    pending_scroll = Some(egui::vec2(0.0, -50.0));
                }
                if ui.button("\u{25BC}").on_hover_text("Down").clicked() {
                    pending_scroll = Some(egui::vec2(0.0, 50.0));
                }
            });
            ui.separator();

            if show_error {
                if let Some(err) = &err_text {
                    ui.colored_label(Color32::RED, err);
                }
                if has_pdf_path {
                    ui.add_space(8.0);
                    if ui.button("Open PDF Externally").clicked() {
                        open_externally = true;
                    }
                }
                return;
            }

            if let Some(tex) = tex {
                let img_size = tex.size_vec2();
                let image = Image::from_texture(
                    egui::load::SizedTexture::new(tex.id(), img_size),
                );

                scroll_offset += pending_scroll.unwrap_or_default();

                let output = ScrollArea::both()
                    .scroll_source(egui::containers::scroll_area::ScrollSource::MOUSE_WHEEL)
                    .vertical_scroll_offset(scroll_offset.y)
                    .horizontal_scroll_offset(scroll_offset.x)
                    .show(ui, |ui| {
                        ui.set_min_size(img_size);
                        ui.add(image);
                    });

                scroll_offset = output.state.offset;
                let _ = output.inner;
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

        if open_externally {
            self.active_tab_mut().preview.open_externally();
        }

        if do_render {
            let tab = self.active_tab_mut();
            tab.preview.zoom = new_zoom;
            tab.preview.page = new_page;
            if let Some(path) = tab.preview.last_pdf_path.clone() {
                tab.preview.render_pdf(&path, new_page);
            }
        }
    }
}
