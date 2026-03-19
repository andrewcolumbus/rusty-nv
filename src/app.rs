use crate::export::{self, ExportFormat};
use crate::import;
use crate::note::{NoteStore, SortField};
use crate::search::search_notes;
use crate::shortcuts;
use crate::storage::{
    self, create_note_file, delete_note_file, load_note_from_file, load_notes_from_dir,
    rename_note_file, save_meta, save_note, FsChange, FsWatcher, NoteMeta,
};
use crate::theme::ThemeChoice;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusTarget {
    None,
    Search,
    Editor,
    Tags,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub theme: ThemeChoice,
    pub horizontal_layout: bool,
    pub notes_dir: Option<String>,
    #[serde(default = "default_confirm_delete")]
    pub confirm_delete: bool,
    #[serde(default = "default_editor_font_size")]
    pub editor_font_size: f32,
    #[serde(default = "default_auto_indent")]
    pub auto_indent: bool,
    #[serde(default = "default_show_tags_column")]
    pub show_tags_column: bool,
    #[serde(default)]
    pub show_date_columns: bool,
}

fn default_confirm_delete() -> bool {
    true
}
fn default_editor_font_size() -> f32 {
    14.0
}
fn default_auto_indent() -> bool {
    true
}
fn default_show_tags_column() -> bool {
    true
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: ThemeChoice::Mocha,
            horizontal_layout: true,
            notes_dir: None,
            confirm_delete: true,
            editor_font_size: 14.0,
            auto_indent: true,
            show_tags_column: true,
            show_date_columns: false,
        }
    }
}

pub struct NvApp {
    pub store: NoteStore,
    pub search_query: String,
    pub filtered_indices: Vec<usize>,
    pub selected_index: Option<usize>,
    pub focus: FocusTarget,
    pub settings: AppSettings,
    pub notes_dir: PathBuf,

    // Dialog state
    pub show_rename_dialog: bool,
    pub show_delete_dialog: bool,
    pub rename_buffer: String,
    pub dialog_error: Option<String>,

    // Auto-save
    last_save: Instant,

    // File watcher
    watcher: Option<FsWatcher>,

    // Wiki link autocomplete: (start_char, end_char, replacement text)
    pub pending_completion: Option<(usize, usize, String)>,

    // Settings window
    pub show_settings: bool,

    // Sorting
    pub sort_field: SortField,
    pub sort_ascending: bool,

    // Multi-select
    pub selected_indices: HashSet<usize>,

    // Tag editing state
    pub tag_input: String,
    pub tag_suggestions: Vec<String>,
    pub show_tag_suggestions: bool,

    // Find in editor (search highlighting + find next/prev)
    /// Which search match is active (0-based), None = all matches shown equally
    pub find_match_index: Option<usize>,
    /// Total number of search matches in the current note's content
    pub find_match_count: usize,
    /// Char offset to jump cursor to on the next frame (set by find_next/find_prev)
    pub pending_cursor_move: Option<usize>,
    /// Previous search query, used to detect when the query changes
    prev_search_query: String,
}

impl NvApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Load persisted settings
        let settings: AppSettings = cc
            .storage
            .and_then(|s| eframe::get_value(s, "nv_settings"))
            .unwrap_or_default();

        // Determine notes directory
        let notes_dir = settings
            .notes_dir
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(storage::default_notes_dir);

        // Apply theme
        settings.theme.apply(&cc.egui_ctx);

        // Load notes
        let store = load_notes_from_dir(&notes_dir);

        // Start file watcher
        let watcher = match FsWatcher::new(&notes_dir) {
            Ok(w) => Some(w),
            Err(e) => {
                error!("File watcher failed: {}", e);
                None
            }
        };

        let mut app = Self {
            store,
            search_query: String::new(),
            filtered_indices: Vec::new(),
            selected_index: None,
            focus: FocusTarget::Search,
            settings,
            notes_dir,
            show_rename_dialog: false,
            show_delete_dialog: false,
            rename_buffer: String::new(),
            dialog_error: None,
            last_save: Instant::now(),
            watcher,
            pending_completion: None,
            show_settings: false,
            sort_field: SortField::default(),
            sort_ascending: false,
            selected_indices: HashSet::new(),
            tag_input: String::new(),
            tag_suggestions: Vec::new(),
            show_tag_suggestions: false,
            find_match_index: None,
            find_match_count: 0,
            pending_cursor_move: None,
            prev_search_query: String::new(),
        };

        app.update_search();
        app
    }

    /// Re-run search and update filtered_indices, then apply sorting.
    pub fn update_search(&mut self) {
        self.filtered_indices = search_notes(&self.store, &self.search_query);

        // Reset find match index when the search query changes
        if self.search_query != self.prev_search_query {
            self.find_match_index = None;
            self.find_match_count = 0;
            self.prev_search_query = self.search_query.clone();
        }

        // When there's no active search query, apply user-chosen sort
        // When searching, keep relevance-based ordering from search_notes
        if self.search_query.is_empty() {
            self.apply_sort();
        }

        // Keep selection if still in filtered results, otherwise select first
        if let Some(sel) = self.selected_index {
            if !self.filtered_indices.contains(&sel) {
                self.selected_index = self.filtered_indices.first().copied();
            }
        } else {
            self.selected_index = self.filtered_indices.first().copied();
        }
    }

    /// Sort filtered_indices by the current sort_field and sort_ascending.
    pub fn apply_sort(&mut self) {
        let store = &self.store;
        let sort_field = self.sort_field;
        let ascending = self.sort_ascending;

        self.filtered_indices.sort_by(|&a, &b| {
            let note_a = store.get(a);
            let note_b = store.get(b);

            let cmp = match (note_a, note_b) {
                (Some(na), Some(nb)) => match sort_field {
                    SortField::Title => na.title_lower.cmp(&nb.title_lower),
                    SortField::DateModified => na.modified.cmp(&nb.modified),
                    SortField::DateCreated => na.created.cmp(&nb.created),
                },
                _ => std::cmp::Ordering::Equal,
            };

            if ascending {
                cmp
            } else {
                cmp.reverse()
            }
        });
    }

    /// Select a note by store index.
    pub fn select_note(&mut self, index: Option<usize>) {
        self.selected_index = index;
        if index.is_some() {
            self.focus = FocusTarget::Editor;
        }
    }

    /// Handle Enter in search field.
    pub fn on_search_enter(&mut self) {
        let query = self.search_query.trim().to_string();
        if query.is_empty() {
            return;
        }

        // If there's an exact title match, select it
        if let Some(idx) = self.store.find_by_title(&query) {
            self.select_note(Some(idx));
            return;
        }

        // If there are results, select the top match
        if let Some(&first) = self.filtered_indices.first() {
            // Check if user intends to create (no close match)
            let top_title = self
                .store
                .get(first)
                .map(|n| n.title_lower.clone())
                .unwrap_or_default();
            if top_title.starts_with(&query.to_lowercase()) {
                self.select_note(Some(first));
                return;
            }
        }

        // Create new note with query as title
        self.create_note(&query);
    }

    /// Create a new note.
    pub fn create_note(&mut self, title: &str) {
        match create_note_file(&self.notes_dir, title) {
            Ok(note) => {
                self.store.add(note);
                let idx = self.store.len() - 1;
                self.update_search();
                self.select_note(Some(idx));
                self.search_query.clear();
                self.update_search();
                info!("Created note: {}", title);
            }
            Err(e) => {
                error!("Failed to create note: {}", e);
            }
        }
    }

    /// Begin rename flow.
    pub fn start_rename(&mut self) {
        if let Some(idx) = self.selected_index {
            if let Some(note) = self.store.get(idx) {
                self.rename_buffer = note.title.clone();
                self.show_rename_dialog = true;
                self.dialog_error = None;
            }
        }
    }

    /// Execute rename.
    pub fn do_rename(&mut self) {
        let new_title = self.rename_buffer.trim().to_string();
        if new_title.is_empty() {
            self.dialog_error = Some("Title cannot be empty".to_string());
            return;
        }

        if let Some(idx) = self.selected_index {
            let notes_dir = self.notes_dir.clone();
            if let Some(note) = self.store.get_mut(idx) {
                match rename_note_file(note, &notes_dir, &new_title) {
                    Ok(()) => {
                        self.store.rebuild_index();
                        self.show_rename_dialog = false;
                        self.dialog_error = None;
                        self.update_search();
                        info!("Renamed note to: {}", new_title);
                    }
                    Err(e) => {
                        self.dialog_error = Some(e);
                    }
                }
            }
        }
    }

    /// Begin delete flow. If confirm_delete is disabled, delete immediately.
    pub fn start_delete(&mut self) {
        if self.selected_index.is_some() {
            if self.settings.confirm_delete {
                self.show_delete_dialog = true;
                self.dialog_error = None;
            } else {
                self.do_delete();
            }
        }
    }

    /// Execute delete.
    pub fn do_delete(&mut self) {
        if let Some(idx) = self.selected_index {
            let note = self.store.get(idx).unwrap();
            match delete_note_file(note) {
                Ok(()) => {
                    let title = note.title.clone();
                    self.store.remove(idx);
                    self.selected_index = None;
                    self.show_delete_dialog = false;
                    self.dialog_error = None;
                    self.update_search();
                    info!("Deleted note: {}", title);
                }
                Err(e) => {
                    self.dialog_error = Some(e);
                }
            }
        }
    }

    /// Auto-save dirty notes and dirty tags sidecar (called every frame, throttled to 1s).
    fn auto_save(&mut self) {
        if self.last_save.elapsed().as_secs_f32() < 1.0 {
            return;
        }
        self.last_save = Instant::now();

        for note in &mut self.store.notes {
            if note.dirty {
                match save_note(note) {
                    Ok(()) => {
                        if let Some(ref mut w) = self.watcher {
                            w.mark_saved(&note.path);
                        }
                        note.mark_saved();
                    }
                    Err(e) => {
                        error!("{}", e);
                    }
                }
            }

            // Save sidecar metadata when tags are dirty
            if note.tags_dirty {
                let meta = NoteMeta {
                    tags: note.tags.clone(),
                };
                match save_meta(&note.path, &meta) {
                    Ok(()) => {
                        note.mark_tags_saved();
                    }
                    Err(e) => {
                        error!("Failed to save tags sidecar: {}", e);
                    }
                }
            }
        }
    }

    /// Process filesystem watcher events.
    fn process_fs_events(&mut self) {
        let changes = match self.watcher.as_mut() {
            Some(w) => w.drain_events(),
            None => return,
        };

        let mut needs_refresh = false;

        for change in changes {
            match change {
                FsChange::Created(path) => {
                    if self.store.notes.iter().any(|n| n.path == path) {
                        continue;
                    }
                    if let Some(note) = load_note_from_file(&path) {
                        info!("External create detected: {}", note.title);
                        self.store.add(note);
                        needs_refresh = true;
                    }
                }
                FsChange::Modified(path) => {
                    if let Some(idx) = self.store.notes.iter().position(|n| n.path == path) {
                        if let Some(new_note) = load_note_from_file(&path) {
                            let note = self.store.get_mut(idx).unwrap();
                            if !note.dirty {
                                info!("External modify detected: {}", note.title);
                                note.content = new_note.content;
                                note.content_lower = new_note.content_lower;
                                note.modified = new_note.modified;
                                needs_refresh = true;
                            }
                        }
                    }
                }
                FsChange::Removed(path) => {
                    if let Some(idx) = self.store.notes.iter().position(|n| n.path == path) {
                        let title = self.store.get(idx).unwrap().title.clone();
                        info!("External delete detected: {}", title);
                        self.store.remove(idx);
                        if self.selected_index == Some(idx) {
                            self.selected_index = None;
                        }
                        needs_refresh = true;
                    }
                }
            }
        }

        if needs_refresh {
            self.update_search();
        }
    }

    /// Handle keyboard shortcuts.
    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        ctx.input_mut(|input| {
            if input.consume_shortcut(&shortcuts::FOCUS_SEARCH) {
                self.focus = FocusTarget::Search;
            }

            if input.consume_shortcut(&shortcuts::NAV_DOWN) {
                self.navigate_list(1);
            }

            if input.consume_shortcut(&shortcuts::NAV_UP) {
                self.navigate_list(-1);
            }

            if input.consume_shortcut(&shortcuts::NEW_NOTE) {
                self.search_query.clear();
                self.focus = FocusTarget::Search;
            }

            if input.consume_shortcut(&shortcuts::RENAME_NOTE) {
                self.start_rename();
            }

            if input.consume_shortcut(&shortcuts::DELETE_NOTE) {
                self.start_delete();
            }

            if input.consume_shortcut(&shortcuts::PASTE_AS_NOTE) {
                self.paste_as_note();
            }

            if input.consume_shortcut(&shortcuts::OPEN_SETTINGS) {
                self.show_settings = !self.show_settings;
            }

            // Import/Export/Print shortcuts (Stream 6)
            if input.consume_shortcut(&shortcuts::IMPORT) {
                self.import_file_dialog();
            }

            if input.consume_shortcut(&shortcuts::EXPORT) {
                self.export_file_dialog();
            }

            if input.consume_shortcut(&shortcuts::PRINT) {
                self.print_current_note();
            }

            // BOOKMARK_TOGGLE must be checked before DESELECT since it uses
            // Cmd+Shift+D (more specific) vs Cmd+D
            if input.consume_shortcut(&shortcuts::BOOKMARK_TOGGLE) {
                self.toggle_bookmark();
            }

            if input.consume_shortcut(&shortcuts::DESELECT) {
                self.selected_index = None;
                self.selected_indices.clear();
                self.focus = FocusTarget::Search;
            }

            // Cmd+1 through Cmd+9 for bookmark jumps
            let number_keys = [
                egui::Key::Num1,
                egui::Key::Num2,
                egui::Key::Num3,
                egui::Key::Num4,
                egui::Key::Num5,
                egui::Key::Num6,
                egui::Key::Num7,
                egui::Key::Num8,
                egui::Key::Num9,
            ];
            for (i, key) in number_keys.iter().enumerate() {
                let shortcut = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, *key);
                if input.consume_shortcut(&shortcut) {
                    self.goto_bookmark((i + 1) as u8);
                }
            }

            // FIND_PREV must be checked before FIND_NEXT since it uses
            // Cmd+Shift+G (more specific) vs Cmd+G
            if input.consume_shortcut(&shortcuts::FIND_PREV) {
                self.find_prev();
            }

            if input.consume_shortcut(&shortcuts::FIND_NEXT) {
                self.find_next();
            }

            if input.consume_shortcut(&shortcuts::FOCUS_TAGS) && self.selected_index.is_some() {
                self.focus = FocusTarget::Tags;
            }

            if input.key_pressed(egui::Key::Escape) {
                self.show_tag_suggestions = false;
                self.search_query.clear();
                self.update_search();
                self.focus = FocusTarget::Search;
            }
        });
    }

    /// Navigate the note list by offset (+1 = down, -1 = up).
    fn navigate_list(&mut self, offset: i32) {
        if self.filtered_indices.is_empty() {
            return;
        }

        let current_pos = self
            .selected_index
            .and_then(|sel| self.filtered_indices.iter().position(|&i| i == sel))
            .unwrap_or(0);

        let new_pos = (current_pos as i32 + offset)
            .max(0)
            .min(self.filtered_indices.len() as i32 - 1) as usize;

        self.selected_index = Some(self.filtered_indices[new_pos]);
    }

    /// Move to the next search match in the current note's content.
    pub fn find_next(&mut self) {
        if self.find_match_count == 0 || self.search_query.is_empty() {
            return;
        }

        let new_index = match self.find_match_index {
            Some(idx) => (idx + 1) % self.find_match_count,
            None => 0,
        };
        self.find_match_index = Some(new_index);
        self.set_pending_cursor_for_match(new_index);
    }

    /// Move to the previous search match in the current note's content.
    pub fn find_prev(&mut self) {
        if self.find_match_count == 0 || self.search_query.is_empty() {
            return;
        }

        let new_index = match self.find_match_index {
            Some(idx) => {
                if idx == 0 {
                    self.find_match_count - 1
                } else {
                    idx - 1
                }
            }
            None => self.find_match_count.saturating_sub(1),
        };
        self.find_match_index = Some(new_index);
        self.set_pending_cursor_for_match(new_index);
    }

    /// Compute the char offset for the given match index and set pending_cursor_move.
    fn set_pending_cursor_for_match(&mut self, match_index: usize) {
        use crate::highlight::{byte_offset_to_char_offset, find_search_matches};

        let content = match self.selected_index.and_then(|i| self.store.get(i)) {
            Some(note) => note.content.clone(),
            None => return,
        };

        let matches = find_search_matches(&content, &self.search_query);
        if let Some(m) = matches.get(match_index) {
            let char_offset = byte_offset_to_char_offset(&content, m.start);
            self.pending_cursor_move = Some(char_offset);
        }
    }

    /// Copy text to the system clipboard.
    pub fn copy_to_clipboard(text: &str) {
        match arboard::Clipboard::new() {
            Ok(mut cb) => {
                if let Err(e) = cb.set_text(text.to_string()) {
                    warn!("Failed to copy to clipboard: {}", e);
                }
            }
            Err(e) => {
                warn!("Failed to access clipboard: {}", e);
            }
        }
    }

    /// Open the file manager with the note's file selected.
    pub fn show_in_file_manager(path: &Path) {
        if let Err(e) = opener::reveal(path) {
            warn!("Failed to show in file manager: {}", e);
        }
    }

    /// Open a note in the system's default text editor.
    pub fn open_in_external_editor(path: &Path) {
        if let Err(e) = opener::open(path) {
            warn!("Failed to open in external editor: {}", e);
        }
    }

    /// Navigate to a wiki link target. If a note with that title exists
    /// (case-insensitive), select it. Otherwise, create a new note with that title.
    pub fn navigate_to_link(&mut self, target: &str) {
        let target_trimmed = target.trim();
        if target_trimmed.is_empty() {
            return;
        }

        // Try to find an existing note by title (case-insensitive)
        if let Some(idx) = self.store.find_by_title(target_trimmed) {
            self.select_note(Some(idx));
            self.search_query.clear();
            self.update_search();
            info!("Navigated to existing note: {}", target_trimmed);
        } else {
            // Create a new note with the link target as title
            self.create_note(target_trimmed);
            info!("Created and navigated to new note: {}", target_trimmed);
        }
    }

    /// Reload notes from a new directory and restart the filesystem watcher.
    pub fn reload_notes_dir(&mut self, new_dir: PathBuf) {
        // Update settings
        self.settings.notes_dir = Some(new_dir.display().to_string());
        self.notes_dir = new_dir.clone();

        // Reload notes from the new directory
        self.store = load_notes_from_dir(&new_dir);

        // Restart filesystem watcher
        self.watcher = match FsWatcher::new(&new_dir) {
            Ok(w) => Some(w),
            Err(e) => {
                error!("File watcher failed for new dir: {}", e);
                None
            }
        };

        // Reset selection and refresh search
        self.selected_index = None;
        self.search_query.clear();
        self.update_search();
        info!("Switched notes directory to: {:?}", new_dir);
    }

    /// Toggle bookmark on the currently selected note.
    /// If bookmarking, assigns the next free slot (1-9).
    pub fn toggle_bookmark(&mut self) {
        if let Some(idx) = self.selected_index {
            if let Some(note) = self.store.get(idx) {
                if note.bookmarked {
                    // Unbookmark
                    let note = self.store.get_mut(idx).unwrap();
                    note.bookmarked = false;
                    note.bookmark_slot = None;
                    info!("Unbookmarked note: {}", note.title);
                } else {
                    // Find next free slot (1-9)
                    let used_slots: HashSet<u8> = self
                        .store
                        .notes
                        .iter()
                        .filter_map(|n| n.bookmark_slot)
                        .collect();
                    let free_slot = (1u8..=9).find(|s| !used_slots.contains(s));

                    let note = self.store.get_mut(idx).unwrap();
                    note.bookmarked = true;
                    note.bookmark_slot = free_slot;
                    info!(
                        "Bookmarked note: {} (slot {:?})",
                        note.title, note.bookmark_slot
                    );
                }
            }
        }
    }

    /// Jump to a bookmarked note by its slot number (1-9).
    pub fn goto_bookmark(&mut self, slot: u8) {
        if let Some(idx) = self
            .store
            .notes
            .iter()
            .position(|n| n.bookmark_slot == Some(slot))
        {
            self.select_note(Some(idx));
            self.search_query.clear();
            self.update_search();
            info!("Jumped to bookmark slot {}", slot);
        }
    }

    /// Delete all notes in the multi-select set.
    pub fn delete_selected(&mut self) {
        if self.selected_indices.is_empty() {
            return;
        }

        // Sort indices in reverse order so removal doesn't invalidate earlier indices
        let mut indices: Vec<usize> = self.selected_indices.iter().copied().collect();
        indices.sort_unstable_by(|a, b| b.cmp(a));

        for idx in indices {
            if let Some(note) = self.store.get(idx) {
                match delete_note_file(note) {
                    Ok(()) => {
                        let title = note.title.clone();
                        self.store.remove(idx);
                        info!("Bulk deleted note: {}", title);
                    }
                    Err(e) => {
                        error!("Failed to bulk delete: {}", e);
                    }
                }
            }
        }

        self.selected_indices.clear();
        self.selected_index = None;
        self.update_search();
    }

    /// Tab auto-complete: find first note whose title starts with the
    /// current search query and replace the query with that title.
    pub fn tab_autocomplete_search(&mut self) {
        let query_lower = self.search_query.to_lowercase();
        if query_lower.is_empty() {
            return;
        }

        // Find the first note (in filtered order) whose title starts with the query
        let match_idx = self.filtered_indices.iter().find(|&&idx| {
            self.store
                .get(idx)
                .map(|n| n.title_lower.starts_with(&query_lower))
                .unwrap_or(false)
        });

        if let Some(&idx) = match_idx {
            if let Some(note) = self.store.get(idx) {
                self.search_query = note.title.clone();
                self.update_search();
                self.select_note(Some(idx));
            }
        }
    }

    /// Update tag autocomplete suggestions based on current tag_input.
    pub fn update_tag_suggestions(&mut self) {
        if self.tag_input.trim().is_empty() {
            self.tag_suggestions.clear();
            self.show_tag_suggestions = false;
            return;
        }

        let input_lower = self.tag_input.trim().to_lowercase();
        let all_tags = self.store.all_tags();

        // Get current note's existing tags to exclude them from suggestions
        let existing_lower: Vec<String> = self
            .selected_index
            .and_then(|idx| self.store.get(idx))
            .map(|note| note.tags_lower.clone())
            .unwrap_or_default();

        self.tag_suggestions = all_tags
            .into_iter()
            .filter(|t| {
                let tl = t.to_lowercase();
                tl.contains(&input_lower) && !existing_lower.contains(&tl)
            })
            .take(8)
            .map(|t| t.to_string())
            .collect();

        self.show_tag_suggestions = !self.tag_suggestions.is_empty();
    }

    /// Add a tag to the currently selected note.
    pub fn add_tag_to_selected(&mut self, tag: &str) {
        if let Some(idx) = self.selected_index {
            if let Some(note) = self.store.get_mut(idx) {
                note.add_tag(tag);
            }
            self.store.rebuild_tag_index();
        }
        self.tag_input.clear();
        self.show_tag_suggestions = false;
    }

    /// Remove a tag from the currently selected note.
    pub fn remove_tag_from_selected(&mut self, tag: &str) {
        if let Some(idx) = self.selected_index {
            if let Some(note) = self.store.get_mut(idx) {
                note.remove_tag(tag);
            }
            self.store.rebuild_tag_index();
        }
    }

    // ─── Import / Export / Print (Stream 6) ─────────────────────────────────

    /// Open a file picker dialog and import the selected file as a new note.
    pub fn import_file_dialog(&mut self) {
        let dialog = rfd::FileDialog::new()
            .set_title("Import File as Note")
            .add_filter("Supported files", &["txt", "md", "html", "htm", "csv"])
            .add_filter("Text files", &["txt", "md"])
            .add_filter("HTML files", &["html", "htm"])
            .add_filter("CSV files", &["csv"])
            .add_filter("All files", &["*"]);

        if let Some(path) = dialog.pick_file() {
            self.import_from_path(&path);
        }
    }

    /// Import a file from a given path, creating a new note from its contents.
    pub fn import_from_path(&mut self, path: &Path) {
        match import::import_file(path) {
            Ok(imported) => match create_note_file(&self.notes_dir, &imported.title) {
                Ok(mut note) => {
                    if !imported.content.is_empty() {
                        note.update_content(imported.content);
                    }
                    self.store.add(note);
                    let idx = self.store.len() - 1;
                    self.update_search();
                    self.select_note(Some(idx));
                    info!("Imported file as note: {}", imported.title);
                }
                Err(e) => {
                    error!("Failed to create note from import: {}", e);
                }
            },
            Err(e) => {
                error!("Failed to import file: {}", e);
            }
        }
    }

    /// Open a save dialog and export the currently selected note.
    pub fn export_file_dialog(&mut self) {
        let (title, content) = match self.selected_index.and_then(|i| self.store.get(i)) {
            Some(note) => (note.title.clone(), note.content.clone()),
            None => return,
        };

        // Let the user choose format via file extension filter
        let dialog = rfd::FileDialog::new()
            .set_title("Export Note")
            .set_file_name(&title)
            .add_filter("HTML (.html)", &["html"])
            .add_filter("Markdown (.md)", &["md"])
            .add_filter("Plain Text (.txt)", &["txt"]);

        if let Some(dest) = dialog.save_file() {
            let format = match dest.extension().and_then(|e| e.to_str()) {
                Some("html") | Some("htm") => ExportFormat::Html,
                Some("md") => ExportFormat::Markdown,
                _ => ExportFormat::PlainText,
            };

            match export::export_note(&title, &content, format, &dest) {
                Ok(()) => {
                    info!("Exported note '{}' to {:?}", title, dest);
                }
                Err(e) => {
                    error!("Failed to export note: {}", e);
                }
            }
        }
    }

    /// Export a specific note by store index (used from context menu).
    pub fn export_note_by_index(&mut self, idx: usize) {
        let (title, content) = match self.store.get(idx) {
            Some(note) => (note.title.clone(), note.content.clone()),
            None => return,
        };

        let dialog = rfd::FileDialog::new()
            .set_title("Export Note")
            .set_file_name(&title)
            .add_filter("HTML (.html)", &["html"])
            .add_filter("Markdown (.md)", &["md"])
            .add_filter("Plain Text (.txt)", &["txt"]);

        if let Some(dest) = dialog.save_file() {
            let format = match dest.extension().and_then(|e| e.to_str()) {
                Some("html") | Some("htm") => ExportFormat::Html,
                Some("md") => ExportFormat::Markdown,
                _ => ExportFormat::PlainText,
            };

            match export::export_note(&title, &content, format, &dest) {
                Ok(()) => {
                    info!("Exported note '{}' to {:?}", title, dest);
                }
                Err(e) => {
                    error!("Failed to export note: {}", e);
                }
            }
        }
    }

    /// Print the current note by generating a temp HTML file and opening it in the browser.
    pub fn print_current_note(&self) {
        let (title, content) = match self.selected_index.and_then(|i| self.store.get(i)) {
            Some(note) => (note.title.clone(), note.content.clone()),
            None => return,
        };

        let html = export::export_to_html(&title, &content);

        // Sanitize filename for temp path
        let safe_name: String = title
            .chars()
            .filter(|c| !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
            .collect();
        let tmp_path = std::env::temp_dir().join(format!("{}.html", safe_name));

        match std::fs::write(&tmp_path, html) {
            Ok(()) => {
                if let Err(e) = opener::open(&tmp_path) {
                    error!("Failed to open print preview: {}", e);
                } else {
                    info!("Opened print preview for '{}'", title);
                }
            }
            Err(e) => {
                error!("Failed to write temp HTML for printing: {}", e);
            }
        }
    }

    /// Paste clipboard contents as a new note.
    fn paste_as_note(&mut self) {
        let clipboard_text = match arboard::Clipboard::new() {
            Ok(mut cb) => match cb.get_text() {
                Ok(text) => text,
                Err(_) => return,
            },
            Err(_) => return,
        };

        if clipboard_text.trim().is_empty() {
            return;
        }

        // Use first line as title, rest as content
        let mut lines = clipboard_text.lines();
        let title = lines
            .next()
            .unwrap_or("Pasted Note")
            .chars()
            .take(50)
            .collect::<String>();
        let content = lines.collect::<Vec<_>>().join("\n");

        match create_note_file(&self.notes_dir, &title) {
            Ok(mut note) => {
                if !content.is_empty() {
                    note.update_content(content);
                }
                self.store.add(note);
                let idx = self.store.len() - 1;
                self.update_search();
                self.select_note(Some(idx));
                info!("Pasted clipboard as note: {}", title);
            }
            Err(e) => {
                error!("Failed to paste as note: {}", e);
            }
        }
    }
}

impl eframe::App for NvApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process background tasks
        self.auto_save();
        self.process_fs_events();
        self.handle_shortcuts(ctx);

        // Request repaint for auto-save and fs watcher polling
        ctx.request_repaint_after(std::time::Duration::from_millis(500));

        // Handle drag-and-drop file import
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        for file in dropped {
            if let Some(path) = &file.path {
                self.import_from_path(path);
            }
        }

        // Show "Drop to import" overlay when files are hovering
        let files_hovering = ctx.input(|i| !i.raw.hovered_files.is_empty());
        if files_hovering {
            #[allow(deprecated)]
            let screen_rect = ctx.screen_rect();
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("drop_overlay"),
            ));
            painter.rect_filled(screen_rect, 0.0, egui::Color32::from_black_alpha(160));
            painter.text(
                screen_rect.center(),
                egui::Align2::CENTER_CENTER,
                "Drop to import",
                egui::FontId::proportional(28.0),
                egui::Color32::WHITE,
            );
        }

        // Draw UI
        self.show_search_bar(ctx);
        self.show_status_bar(ctx);
        self.show_dialogs(ctx);
        self.show_settings_window(ctx);
        self.show_note_list_panel(ctx);
        self.show_editor(ctx);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "nv_settings", &self.settings);
    }
}
