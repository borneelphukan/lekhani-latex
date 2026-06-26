use egui::Color32;
use crate::app::App;
use crate::compiler::CompileStatus;

impl App {
    pub(crate) fn status_bar(&mut self, ui: &mut egui::Ui) {
        if self.tabs.is_empty() {
            ui.horizontal(|ui| {
                ui.colored_label(Color32::GRAY, "Ready");
            });
            return;
        }

        let tab = self.active_tab();
        let (line, col) = tab.buffer.cursor_line_col();
        let path = tab
            .buffer
            .path()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "Untitled".into());
        let dirty = if tab.buffer.dirty { " •" } else { "" };
        let compile_status = match tab.compiler.status() {
            CompileStatus::Idle => "",
            CompileStatus::Running => " | Compiling…",
            CompileStatus::Success => " | OK",
            CompileStatus::Failed => " | Error",
        };

        ui.horizontal(|ui| {
            ui.colored_label(
                Color32::GRAY,
                format!(
                    "{}{} | Ln {}, Col {} | {}{}",
                    path, dirty, line, col + 1, tab.status_message, compile_status,
                ),
            );
        });
    }
}
