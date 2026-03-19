use crate::app::NvApp;
use crate::shortcuts;
use crate::theme::ThemeChoice;
use std::path::PathBuf;

impl NvApp {
    pub fn show_settings_window(&mut self, ctx: &egui::Context) {
        if !self.show_settings {
            return;
        }

        let mut open = self.show_settings;
        let mut new_dir: Option<PathBuf> = None;
        let mut theme_changed = false;

        egui::Window::new("Preferences")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(480.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    // ── General ──────────────────────────────────────────
                    ui.heading("General");
                    ui.separator();
                    ui.add_space(4.0);

                    egui::Grid::new("settings_general_grid")
                        .num_columns(2)
                        .spacing([12.0, 8.0])
                        .show(ui, |ui| {
                            ui.label("Notes directory:");
                            ui.horizontal(|ui| {
                                let dir_display = self.notes_dir.display().to_string();
                                ui.add(
                                    egui::TextEdit::singleline(&mut dir_display.clone())
                                        .desired_width(280.0)
                                        .interactive(false),
                                );
                                if ui.button("Browse...").clicked() {
                                    if let Some(path) = rfd::FileDialog::new()
                                        .set_directory(&self.notes_dir)
                                        .pick_folder()
                                    {
                                        new_dir = Some(path);
                                    }
                                }
                            });
                            ui.end_row();

                            ui.label("Confirm on delete:");
                            ui.checkbox(
                                &mut self.settings.confirm_delete,
                                "Ask before deleting notes",
                            );
                            ui.end_row();
                        });

                    ui.add_space(12.0);

                    // ── Editor ───────────────────────────────────────────
                    ui.heading("Editor");
                    ui.separator();
                    ui.add_space(4.0);

                    egui::Grid::new("settings_editor_grid")
                        .num_columns(2)
                        .spacing([12.0, 8.0])
                        .show(ui, |ui| {
                            ui.label("Font size:");
                            ui.add(
                                egui::Slider::new(&mut self.settings.editor_font_size, 8.0..=24.0)
                                    .step_by(1.0)
                                    .suffix(" px"),
                            );
                            ui.end_row();

                            ui.label("Auto-indent:");
                            ui.checkbox(
                                &mut self.settings.auto_indent,
                                "Continue indentation on new lines",
                            );
                            ui.end_row();
                        });

                    ui.add_space(12.0);

                    // ── Display ──────────────────────────────────────────
                    ui.heading("Display");
                    ui.separator();
                    ui.add_space(4.0);

                    egui::Grid::new("settings_display_grid")
                        .num_columns(2)
                        .spacing([12.0, 8.0])
                        .show(ui, |ui| {
                            ui.label("Show tags column:");
                            ui.checkbox(
                                &mut self.settings.show_tags_column,
                                "Display tags in note list",
                            );
                            ui.end_row();

                            ui.label("Show date columns:");
                            ui.checkbox(
                                &mut self.settings.show_date_columns,
                                "Display modified/created dates",
                            );
                            ui.end_row();
                        });

                    ui.add_space(12.0);

                    // ── Themes ───────────────────────────────────────────
                    ui.heading("Themes");
                    ui.separator();
                    ui.add_space(4.0);

                    let current_theme = self.settings.theme;
                    for &choice in ThemeChoice::all() {
                        if ui.radio(current_theme == choice, choice.label()).clicked() {
                            self.settings.theme = choice;
                            theme_changed = true;
                        }
                    }

                    ui.add_space(12.0);

                    // ── Keyboard Shortcuts ───────────────────────────────
                    ui.heading("Keyboard Shortcuts");
                    ui.separator();
                    ui.add_space(4.0);

                    egui::Grid::new("settings_shortcuts_grid")
                        .num_columns(2)
                        .spacing([24.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            shortcut_row(ui, "Focus search", &shortcuts::FOCUS_SEARCH);
                            shortcut_row(ui, "Navigate down", &shortcuts::NAV_DOWN);
                            shortcut_row(ui, "Navigate up", &shortcuts::NAV_UP);
                            shortcut_row(ui, "New note", &shortcuts::NEW_NOTE);
                            shortcut_row(ui, "Rename note", &shortcuts::RENAME_NOTE);
                            shortcut_row(ui, "Delete note", &shortcuts::DELETE_NOTE);
                            shortcut_row(ui, "Paste as note", &shortcuts::PASTE_AS_NOTE);
                            shortcut_row(ui, "Preferences", &shortcuts::OPEN_SETTINGS);
                            shortcut_row(ui, "Import file", &shortcuts::IMPORT);
                            shortcut_row(ui, "Export note", &shortcuts::EXPORT);
                            shortcut_row(ui, "Print note", &shortcuts::PRINT);
                        });

                    ui.add_space(16.0);

                    // ── Close Button ─────────────────────────────────────
                    ui.separator();
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if ui.button("Close").clicked() {
                            self.show_settings = false;
                        }
                    });
                });
            });

        // Apply deferred actions outside the UI closure
        if !open {
            self.show_settings = false;
        }

        if theme_changed {
            self.settings.theme.apply(ctx);
        }

        if let Some(dir) = new_dir {
            self.reload_notes_dir(dir);
        }
    }
}

/// Render a single row in the keyboard shortcuts reference table.
fn shortcut_row(ui: &mut egui::Ui, label: &str, shortcut: &egui::KeyboardShortcut) {
    ui.label(label);
    ui.weak(format_shortcut(shortcut));
    ui.end_row();
}

/// Format a KeyboardShortcut as a human-readable string.
fn format_shortcut(shortcut: &egui::KeyboardShortcut) -> String {
    let mut parts = Vec::new();

    if shortcut.modifiers.command {
        if cfg!(target_os = "macos") {
            parts.push("Cmd");
        } else {
            parts.push("Ctrl");
        }
    }
    if shortcut.modifiers.shift {
        parts.push("Shift");
    }
    if shortcut.modifiers.alt {
        parts.push("Alt");
    }

    parts.push(key_name(shortcut.logical_key));
    parts.join("+")
}

/// Convert an egui Key to a display name.
fn key_name(key: egui::Key) -> &'static str {
    match key {
        egui::Key::L => "L",
        egui::Key::J => "J",
        egui::Key::K => "K",
        egui::Key::N => "N",
        egui::Key::R => "R",
        egui::Key::V => "V",
        egui::Key::D => "D",
        egui::Key::E => "E",
        egui::Key::I => "I",
        egui::Key::P => "P",
        egui::Key::Delete => "Delete",
        egui::Key::Comma => ",",
        _ => "?",
    }
}
