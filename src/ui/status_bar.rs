use crate::app::NvApp;

impl NvApp {
    pub fn show_status_bar(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(22.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let total = self.store.len();
                    let shown = self.filtered_indices.len();

                    if self.search_query.is_empty() {
                        ui.weak(format!("{} notes", total));
                    } else {
                        ui.weak(format!("{}/{} notes", shown, total));
                    }

                    // Show search match count in the current note
                    if !self.search_query.is_empty() && self.find_match_count > 0 {
                        ui.separator();
                        if let Some(idx) = self.find_match_index {
                            ui.weak(format!("Match {}/{}", idx + 1, self.find_match_count));
                        } else {
                            ui.weak(format!("{} matches", self.find_match_count));
                        }
                    }

                    ui.separator();

                    let dirty_count = self.store.notes.iter().filter(|n| n.dirty).count();
                    if dirty_count > 0 {
                        ui.weak(format!("{} unsaved", dirty_count));
                        ui.separator();
                    }

                    ui.weak(self.notes_dir.display().to_string());
                });
            });
    }
}
