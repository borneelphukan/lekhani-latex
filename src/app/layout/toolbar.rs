use egui::Color32;
use crate::app::App;
use crate::types::Theme;

impl App {
    pub(crate) fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let compile_enabled = !self.tabs.is_empty()
                && self.active_tab().buffer.path().is_some();
            
            let is_compiling = compile_enabled && 
                matches!(self.active_tab().compiler.status(), crate::compiler::CompileStatus::Running);
                
            let button_text = if is_compiling {
                "  \u{23F3}  Compiling…  "
            } else {
                "  \u{25B6}  Compile  "
            };

            let resolved = self.theme.resolve(ui.ctx());
            let compile_btn = crate::components::button::toolbar(button_text, match resolved {
                Theme::Dark => Color32::from_rgb(30, 120, 60),
                Theme::Light => Color32::from_rgb(40, 160, 80),
                Theme::System => unreachable!(),
            });
            let resp = ui.add_enabled(compile_enabled && !is_compiling, compile_btn);
            if resp.clicked() {
                self.trigger_compile();
            }

            ui.add_space(8.0);
            
            let llm_btn_text = if self.llm_correction_in_progress {
                "  \u{23F3}  Fixing…  "
            } else {
                "  \u{2728}  Fix with AI  "
            };
            
            let llm_btn = crate::components::button::toolbar(llm_btn_text, match resolved {
                Theme::Dark => Color32::from_rgb(160, 100, 160),
                Theme::Light => Color32::from_rgb(200, 120, 200),
                Theme::System => unreachable!(),
            }).min_size(egui::vec2(110.0, 26.0));
                
            let llm_enabled = !self.tabs.is_empty() && !self.llm_correction_in_progress && self.has_saved_llm_key && self.active_tab().error_message.is_some();
            let resp_llm = ui.add_enabled(llm_enabled, llm_btn);
            if resp_llm.clicked() {
                self.trigger_llm_correction();
            }

            ui.separator();

            ui.checkbox(&mut self.auto_compile, "Auto-compile");
        });
    }
}
