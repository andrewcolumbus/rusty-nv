use crate::app::{FocusTarget, NvApp};

impl NvApp {
    pub fn show_search_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("search_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("🔍");
                let response = ui.add_sized(
                    [ui.available_width() - 100.0, 24.0],
                    egui::TextEdit::singleline(&mut self.search_query)
                        .hint_text("Search or create note…"),
                );

                if self.focus == FocusTarget::Search {
                    response.request_focus();
                    self.focus = FocusTarget::None;
                }

                if response.changed() {
                    self.update_search();
                }

                // Tab: auto-complete with first matching note title
                if response.has_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Tab))
                    && !self.search_query.is_empty()
                {
                    self.tab_autocomplete_search();
                }

                // Enter: select top match or create new note
                if response.lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    && !self.search_query.is_empty()
                {
                    self.on_search_enter();
                }

                // Theme toggle
                if ui.button(self.settings.theme.label()).clicked() {
                    self.settings.theme = self.settings.theme.toggle();
                    self.settings.theme.apply(ctx);
                }
            });
        });
    }
}
