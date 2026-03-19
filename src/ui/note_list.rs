use crate::app::NvApp;
use crate::note::SortField;
use chrono::{DateTime, Local};
use chrono_humanize::HumanTime;
use std::path::PathBuf;

enum ContextAction {
    Select(usize),
    Rename(usize),
    Delete(usize),
    DeleteSelected,
    ToggleBookmark(usize),
    CopyTitle(String),
    CopyPath(String),
    ShowInFileManager(PathBuf),
    OpenExternal(PathBuf),
    ExportNote(usize),
}

/// Format a DateTime using chrono-humanize for relative display.
fn format_relative_time(dt: &DateTime<Local>) -> String {
    let duration = Local::now().signed_duration_since(*dt);
    let ht = HumanTime::from(duration);
    format!("{}", ht)
}

/// Display data for one note row, collected before rendering to avoid borrow issues.
#[allow(dead_code)]
struct NoteListItem {
    store_idx: usize,
    title: String,
    is_primary_selected: bool,
    is_multi_selected: bool,
    path: PathBuf,
    modified_str: String,
    created_str: String,
    bookmarked: bool,
    bookmark_slot: Option<u8>,
    dirty: bool,
    list_position: usize,
    tags_display: String,
}

impl NvApp {
    pub fn show_note_list_panel(&mut self, ctx: &egui::Context) {
        if self.settings.horizontal_layout {
            egui::SidePanel::left("note_list")
                .default_width(300.0)
                .min_width(150.0)
                .show(ctx, |ui| {
                    self.show_note_list(ui);
                });
        } else {
            egui::TopBottomPanel::top("note_list")
                .resizable(true)
                .default_height(200.0)
                .min_height(80.0)
                .show(ctx, |ui| {
                    self.show_note_list(ui);
                });
        }
    }

    fn show_note_list(&mut self, ui: &mut egui::Ui) {
        // Column headers row
        self.show_column_headers(ui);

        ui.separator();

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                if self.filtered_indices.is_empty() {
                    ui.weak("No notes found");
                    return;
                }

                let show_dates = self.settings.show_date_columns;

                // Collect display data first to avoid borrow issues
                let items: Vec<NoteListItem> = self
                    .filtered_indices
                    .iter()
                    .enumerate()
                    .filter_map(|(list_pos, &idx)| {
                        self.store.get(idx).map(|note| {
                            let title = if note.dirty {
                                format!("● {}", note.title)
                            } else {
                                note.title.clone()
                            };
                            let tags_display = if note.tags.is_empty() {
                                String::new()
                            } else {
                                format!(" [{}]", note.tags.join(", "))
                            };
                            NoteListItem {
                                store_idx: idx,
                                title,
                                is_primary_selected: self.selected_index == Some(idx),
                                is_multi_selected: self.selected_indices.contains(&idx),
                                path: note.path.clone(),
                                modified_str: format_relative_time(&note.modified),
                                created_str: format_relative_time(&note.created),
                                bookmarked: note.bookmarked,
                                bookmark_slot: note.bookmark_slot,
                                dirty: note.dirty,
                                list_position: list_pos,
                                tags_display,
                            }
                        })
                    })
                    .collect();

                let mut action: Option<ContextAction> = None;
                let mut shift_click_idx: Option<usize> = None;
                let mut cmd_click_idx: Option<usize> = None;

                for item in &items {
                    let is_selected = item.is_primary_selected || item.is_multi_selected;

                    // Build the row label
                    let show_tags = self.settings.show_tags_column;
                    let row_text = if show_dates {
                        // Bookmark indicator + Title + Tags + Modified + Created
                        let bookmark_indicator = if item.bookmarked {
                            match item.bookmark_slot {
                                Some(slot) => format!("[{}] ", slot),
                                None => "* ".to_string(),
                            }
                        } else {
                            String::new()
                        };
                        let tags_part = if show_tags { &item.tags_display } else { "" };
                        format!(
                            "{}{}{}  |  {}  |  {}",
                            bookmark_indicator,
                            item.title,
                            tags_part,
                            item.modified_str,
                            item.created_str
                        )
                    } else {
                        let bookmark_indicator = if item.bookmarked {
                            match item.bookmark_slot {
                                Some(slot) => format!("[{}] ", slot),
                                None => "* ".to_string(),
                            }
                        } else {
                            String::new()
                        };
                        let tags_part = if show_tags { &item.tags_display } else { "" };
                        format!("{}{}{}", bookmark_indicator, item.title, tags_part)
                    };

                    let response = ui.add_sized(
                        [ui.available_width(), 20.0],
                        egui::Button::new(row_text).selected(is_selected),
                    );

                    if response.clicked() {
                        let modifiers = ui.input(|i| i.modifiers);
                        if modifiers.shift {
                            shift_click_idx = Some(item.list_position);
                        } else if modifiers.command {
                            cmd_click_idx = Some(item.store_idx);
                        } else {
                            action = Some(ContextAction::Select(item.store_idx));
                        }
                    }

                    response.context_menu(|ui| {
                        if ui.button("Rename Note").clicked() {
                            action = Some(ContextAction::Rename(item.store_idx));
                            ui.close();
                        }
                        if ui.button("Delete Note").clicked() {
                            action = Some(ContextAction::Delete(item.store_idx));
                            ui.close();
                        }

                        // Bulk delete option when multi-selected
                        if !self.selected_indices.is_empty() {
                            let count = self.selected_indices.len();
                            if ui.button(format!("Delete {} Selected", count)).clicked() {
                                action = Some(ContextAction::DeleteSelected);
                                ui.close();
                            }
                        }

                        ui.separator();

                        let bookmark_label = if item.bookmarked {
                            "Remove Bookmark"
                        } else {
                            "Add Bookmark"
                        };
                        if ui.button(bookmark_label).clicked() {
                            action = Some(ContextAction::ToggleBookmark(item.store_idx));
                            ui.close();
                        }

                        ui.separator();

                        if ui.button("Copy Title").clicked() {
                            let raw_title = item.title.strip_prefix("● ").unwrap_or(&item.title);
                            action = Some(ContextAction::CopyTitle(raw_title.to_string()));
                            ui.close();
                        }
                        if ui.button("Copy File Path").clicked() {
                            action = Some(ContextAction::CopyPath(item.path.display().to_string()));
                            ui.close();
                        }

                        ui.separator();

                        if ui.button("Show in File Manager").clicked() {
                            action = Some(ContextAction::ShowInFileManager(item.path.clone()));
                            ui.close();
                        }
                        if ui.button("Open in External Editor").clicked() {
                            action = Some(ContextAction::OpenExternal(item.path.clone()));
                            ui.close();
                        }

                        ui.separator();

                        if ui.button("Export as...").clicked() {
                            action = Some(ContextAction::ExportNote(item.store_idx));
                            ui.close();
                        }
                    });
                }

                // Handle Shift+click range selection
                if let Some(click_pos) = shift_click_idx {
                    let anchor_pos = self
                        .selected_index
                        .and_then(|sel| self.filtered_indices.iter().position(|&i| i == sel))
                        .unwrap_or(0);

                    let start = anchor_pos.min(click_pos);
                    let end = anchor_pos.max(click_pos);

                    self.selected_indices.clear();
                    for pos in start..=end {
                        if let Some(&idx) = self.filtered_indices.get(pos) {
                            self.selected_indices.insert(idx);
                        }
                    }
                }

                // Handle Cmd+click toggle selection
                if let Some(idx) = cmd_click_idx {
                    if self.selected_indices.contains(&idx) {
                        self.selected_indices.remove(&idx);
                    } else {
                        self.selected_indices.insert(idx);
                    }
                }

                // Execute deferred action
                if let Some(act) = action {
                    match act {
                        ContextAction::Select(idx) => {
                            self.selected_indices.clear();
                            self.select_note(Some(idx));
                        }
                        ContextAction::Rename(idx) => {
                            self.select_note(Some(idx));
                            self.start_rename();
                        }
                        ContextAction::Delete(idx) => {
                            self.select_note(Some(idx));
                            self.start_delete();
                        }
                        ContextAction::DeleteSelected => {
                            self.delete_selected();
                        }
                        ContextAction::ToggleBookmark(idx) => {
                            self.selected_index = Some(idx);
                            self.toggle_bookmark();
                        }
                        ContextAction::CopyTitle(t) => NvApp::copy_to_clipboard(&t),
                        ContextAction::CopyPath(p) => NvApp::copy_to_clipboard(&p),
                        ContextAction::ShowInFileManager(p) => NvApp::show_in_file_manager(&p),
                        ContextAction::OpenExternal(p) => NvApp::open_in_external_editor(&p),
                        ContextAction::ExportNote(idx) => {
                            self.export_note_by_index(idx);
                        }
                    }
                }
            });
    }

    /// Show clickable column headers for sorting. Right-click to toggle column visibility.
    fn show_column_headers(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Title column header
            let title_label = Self::sort_header_label(
                "Title",
                SortField::Title,
                self.sort_field,
                self.sort_ascending,
            );
            if ui.button(title_label).clicked() {
                self.toggle_sort(SortField::Title);
            }

            // Modified column header
            let mod_label = Self::sort_header_label(
                "Modified",
                SortField::DateModified,
                self.sort_field,
                self.sort_ascending,
            );
            if ui.button(mod_label).clicked() {
                self.toggle_sort(SortField::DateModified);
            }

            // Created column header (only if date columns shown)
            if self.settings.show_date_columns {
                let created_label = Self::sort_header_label(
                    "Created",
                    SortField::DateCreated,
                    self.sort_field,
                    self.sort_ascending,
                );
                if ui.button(created_label).clicked() {
                    self.toggle_sort(SortField::DateCreated);
                }
            }
        });

        // Right-click on the header area to toggle columns
        let header_response = ui.interact(
            ui.min_rect(),
            ui.id().with("column_header_ctx"),
            egui::Sense::click(),
        );
        header_response.context_menu(|ui| {
            if ui
                .checkbox(&mut self.settings.show_date_columns, "Show Date Columns")
                .clicked()
            {
                ui.close();
            }
            if ui
                .checkbox(&mut self.settings.show_tags_column, "Show Tags Column")
                .clicked()
            {
                ui.close();
            }
        });
    }

    /// Build a header label string.
    fn sort_header_label(
        label: &str,
        _field: SortField,
        _current_field: SortField,
        _ascending: bool,
    ) -> String {
        label.to_string()
    }

    /// Toggle sorting: if same field, flip direction; if different field, set descending.
    fn toggle_sort(&mut self, field: SortField) {
        if self.sort_field == field {
            self.sort_ascending = !self.sort_ascending;
        } else {
            self.sort_field = field;
            // Default to descending for dates, ascending for title
            self.sort_ascending = matches!(field, SortField::Title);
        }
        self.update_search();
    }
}
