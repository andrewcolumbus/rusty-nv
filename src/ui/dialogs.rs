use crate::app::NvApp;

impl NvApp {
    pub fn show_dialogs(&mut self, ctx: &egui::Context) {
        self.show_rename_dialog(ctx);
        self.show_delete_dialog(ctx);
    }

    fn show_rename_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_rename_dialog {
            return;
        }

        let mut do_rename = false;
        let mut do_close = false;

        egui::Window::new("Rename Note")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("New name:");
                    let response = ui.text_edit_singleline(&mut self.rename_buffer);
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        do_rename = true;
                    }
                });

                ui.horizontal(|ui| {
                    if ui.button("Rename").clicked() {
                        do_rename = true;
                    }
                    if ui.button("Cancel").clicked() {
                        do_close = true;
                    }
                });

                if let Some(ref err) = self.dialog_error {
                    ui.colored_label(egui::Color32::RED, err);
                }
            });

        if do_rename {
            self.do_rename();
        }

        if do_close {
            self.show_rename_dialog = false;
            self.dialog_error = None;
        }
    }

    fn show_delete_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_delete_dialog {
            return;
        }

        let title = self
            .selected_index
            .and_then(|i| self.store.get(i))
            .map(|n| n.title.clone())
            .unwrap_or_default();

        let mut do_delete = false;
        let mut do_close = false;

        egui::Window::new("Delete Note")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(format!("Delete '{}'?", title));
                ui.label("This cannot be undone.");

                ui.horizontal(|ui| {
                    if ui.button("Delete").clicked() {
                        do_delete = true;
                    }
                    if ui.button("Cancel").clicked() {
                        do_close = true;
                    }
                });

                if let Some(ref err) = self.dialog_error {
                    ui.colored_label(egui::Color32::RED, err);
                }
            });

        if do_delete {
            self.do_delete();
        }

        if do_close {
            self.show_delete_dialog = false;
            self.dialog_error = None;
        }
    }
}
