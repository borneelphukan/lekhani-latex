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
        let has_preview = self.active_tab().preview.image_size.is_some();

        ui.add_enabled_ui(has_preview, |ui| {
            let mut new_zoom = zoom;
            let mut new_page = page;
            
            let btn_size = egui::vec2(28.0, 28.0);

            if ui.add_sized(btn_size, egui::Button::new("−")).clicked() {
                new_zoom = (zoom - 0.25).max(0.25);
            }
            ui.label(format!("{}%", (zoom * 100.0) as u32));
            if ui.add_sized(btn_size, egui::Button::new("+")).clicked() {
                new_zoom = (zoom + 0.25).min(4.0);
            }
            ui.separator();
            
            let has_next = num_pages.map_or(true, |n| page + 1 < n);
            ui.add_enabled_ui(has_next, |ui| {
                if ui.add_sized(btn_size, egui::Button::new("\u{25B6}")).clicked() {
                    new_page = page + 1;
                }
            });
            ui.label(format!("Page {}", page + 1));
            let has_prev = page > 0;
            ui.add_enabled_ui(has_prev, |ui| {
                if ui.add_sized(btn_size, egui::Button::new("\u{25C0}")).clicked() {
                    new_page = page.saturating_sub(1);
                }
            });
            ui.separator();
            
            if ui.add_sized(btn_size, egui::Button::new("\u{25C0}")).on_hover_text("Left").clicked() {
                let offset = self.active_tab().scroll_offset;
                self.active_tab_mut().scroll_request = Some(egui::vec2(offset.x - 50.0, offset.y));
            }
            if ui.add_sized(btn_size, egui::Button::new("\u{25B6}")).on_hover_text("Right").clicked() {
                let offset = self.active_tab().scroll_offset;
                self.active_tab_mut().scroll_request = Some(egui::vec2(offset.x + 50.0, offset.y));
            }
            if ui.add_sized(btn_size, egui::Button::new("\u{25B2}")).on_hover_text("Up").clicked() {
                let offset = self.active_tab().scroll_offset;
                self.active_tab_mut().scroll_request = Some(egui::vec2(offset.x, offset.y - 50.0));
            }
            if ui.add_sized(btn_size, egui::Button::new("\u{25BC}")).on_hover_text("Down").clicked() {
                let offset = self.active_tab().scroll_offset;
                self.active_tab_mut().scroll_request = Some(egui::vec2(offset.x, offset.y + 50.0));
            }

            self.active_tab_mut().preview.zoom = new_zoom;
            if new_page != page {
                self.active_tab_mut().preview.page = new_page;
                if let Some(size) = self.active_tab().preview.image_size {
                    let display_size = egui::vec2(size[0] as f32, size[1] as f32) * new_zoom;
                    self.active_tab_mut().scroll_request = Some(egui::vec2(0.0, new_page as f32 * (display_size.y + 16.0)));
                }
            }
            
            ui.separator();
            
            let mut pan_mode = self.active_tab().preview.pan_mode;
            if ui.add_sized(btn_size, egui::Button::new("↖").selected(!pan_mode)).clicked() {
                pan_mode = false;
            }
            if ui.add_sized(btn_size, egui::Button::new("✋").selected(pan_mode)).clicked() {
                pan_mode = true;
            }
            self.active_tab_mut().preview.pan_mode = pan_mode;
        });
    }

    pub(super) fn preview_content(&mut self, ui: &mut egui::Ui) {
        if self.tabs.is_empty() {
            return;
        }

        let image_size = self.active_tab().preview.image_size;
        let mut zoom = self.active_tab().preview.zoom;
        let mut scroll_offset = self.active_tab().scroll_offset;
        let num_pages = self.active_tab().preview.num_pages.unwrap_or(1);
        let pdf_path = self.active_tab().preview.last_pdf_path.clone();

        let mut zoom_delta = ui.input(|i| i.zoom_delta());
        
        let ctrl_down = ui.input(|i| i.modifiers.command);
        if ctrl_down {
            let scroll_y = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll_y != 0.0 {
                let factor = if scroll_y > 0.0 { 1.1 } else { 0.9 };
                zoom_delta *= factor;
            }
            ui.input_mut(|i| {
                i.events.retain(|e| {
                    !matches!(e, egui::Event::MouseWheel { .. })
                });
            });
        }

        if zoom_delta != 1.0 {
            zoom = (zoom * zoom_delta).clamp(0.25, 4.0);
        }

        let inner = ui.vertical(|ui| {
            if let Some(size) = image_size {
                let img_size = egui::vec2(size[0] as f32, size[1] as f32);

                if zoom == 1.0 && img_size.x > 0.0 {
                    let avail_width = ui.available_width() - 30.0; // Account for scrollbar and borders
                    if avail_width > 0.0 {
                        let fit = avail_width / img_size.x;
                        if fit < 1.0 {
                            zoom = fit.max(0.25).min(4.0);
                        }
                    }
                }

                let display_size = img_size * zoom;
                let page_height = display_size.y + 16.0;

                let mut scroll_area = ScrollArea::both()
                    .auto_shrink([false, false])
                    .scroll_source(egui::containers::scroll_area::ScrollSource::MOUSE_WHEEL);
                    
                if let Some(request) = self.active_tab_mut().scroll_request.take() {
                    scroll_area = scroll_area
                        .vertical_scroll_offset(request.y)
                        .horizontal_scroll_offset(request.x);
                }

                let visible_height = ui.available_height();

                let output = scroll_area.show(ui, |ui| {
                        // Calculate which page is currently most visible
                        let current_page = (scroll_offset.y / page_height).round().max(0.0) as usize;
                        if current_page < num_pages {
                            self.active_tab_mut().preview.page = current_page;
                        }

                        for p in 0..num_pages {
                            let top_y = p as f32 * page_height;
                            let bottom_y = top_y + display_size.y;
                            
                            let viewport_top = scroll_offset.y;
                            let viewport_bottom = scroll_offset.y + visible_height;
                            
                            // Check if page is near the viewport
                            if bottom_y < viewport_top - page_height || top_y > viewport_bottom + page_height {
                                // Add placeholder space for off-screen pages
                                ui.allocate_exact_size(egui::vec2(display_size.x, display_size.y + 16.0), egui::Sense::hover());
                                continue;
                            }

                            // Trigger rendering if needed
                            if !self.active_tab().preview_textures.contains_key(&p) {
                                if let Some(path) = &pdf_path {
                                    self.active_tab_mut().preview.ensure_page_rendered(path, p);
                                }
                            }

                            // Center the page horizontally
                            ui.horizontal(|ui| {
                                ui.add_space((ui.available_width() - display_size.x).max(0.0) / 2.0);
                                
                                let frame = egui::Frame::NONE
                                    .fill(Color32::WHITE)
                                    .shadow(egui::epaint::Shadow {
                                        offset: [0, 2],
                                        blur: 4,
                                        spread: 0,
                                        color: Color32::from_black_alpha(40),
                                    })
                                    .inner_margin(0.0);
                                    
                                frame.show(ui, |ui| {
                                    if let Some(tex) = self.active_tab().preview_textures.get(&p) {
                                        let image = Image::from_texture(
                                            egui::load::SizedTexture::new(tex.id(), display_size),
                                        );
                                        ui.add(image);
                                    } else {
                                        let (_, resp) = ui.allocate_exact_size(display_size, egui::Sense::hover());
                                        ui.painter().text(
                                            resp.rect.center(),
                                            egui::Align2::CENTER_CENTER,
                                            "Rendering...",
                                            egui::FontId::proportional(16.0),
                                            Color32::GRAY,
                                        );
                                    }
                                });
                            });
                            ui.add_space(16.0); // Border between pages
                        }
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

        let (hover_pos, middle_down, primary_down, delta) = ui.input(|i| {
            (
                i.pointer.hover_pos(), 
                i.pointer.middle_down(), 
                i.pointer.primary_down(),
                i.pointer.delta()
            )
        });
        
        let pan_mode = self.active_tab().preview.pan_mode;
        
        if let Some(pos) = hover_pos {
            if preview_rect.contains(pos) {
                if pan_mode {
                    ui.ctx().set_cursor_icon(if primary_down { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
                }
                
                if middle_down || (pan_mode && primary_down) {
                    let new_offset = scroll_offset + delta;
                    self.active_tab_mut().scroll_request = Some(new_offset);
                    if middle_down {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                    }
                }
            }
        }

        self.active_tab_mut().scroll_offset = scroll_offset;
        self.active_tab_mut().preview.zoom = zoom;
    }
}
