use crate::note::{Note, NoteStore};
use chrono::{DateTime, Local};
use log::{error, info, warn};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Instant;

/// Sidecar metadata stored alongside each note as `<filename>.meta.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NoteMeta {
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Get the sidecar metadata path for a note file.
/// e.g. `foo.txt` -> `foo.txt.meta.json`
pub fn meta_path(note_path: &Path) -> PathBuf {
    let mut p = note_path.as_os_str().to_owned();
    p.push(".meta.json");
    PathBuf::from(p)
}

/// Load sidecar metadata for a note. Returns default if missing or malformed.
pub fn load_meta(note_path: &Path) -> NoteMeta {
    let path = meta_path(note_path);
    if !path.exists() {
        return NoteMeta::default();
    }
    match fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => NoteMeta::default(),
    }
}

/// Save sidecar metadata for a note as pretty JSON.
pub fn save_meta(note_path: &Path, meta: &NoteMeta) -> Result<(), String> {
    let path = meta_path(note_path);
    let json = serde_json::to_string_pretty(meta)
        .map_err(|e| format!("Failed to serialize meta: {}", e))?;
    fs::write(&path, json).map_err(|e| format!("Failed to save meta {:?}: {}", path, e))
}

/// Delete the sidecar metadata file for a note (if it exists).
pub fn delete_meta(note_path: &Path) {
    let path = meta_path(note_path);
    if path.exists() {
        if let Err(e) = fs::remove_file(&path) {
            warn!("Failed to delete meta {:?}: {}", path, e);
        }
    }
}

/// Rename (move) the sidecar metadata file when a note is renamed.
pub fn rename_meta(old_note_path: &Path, new_note_path: &Path) {
    let old_meta = meta_path(old_note_path);
    if old_meta.exists() {
        let new_meta = meta_path(new_note_path);
        if let Err(e) = fs::rename(&old_meta, &new_meta) {
            warn!(
                "Failed to rename meta {:?} -> {:?}: {}",
                old_meta, new_meta, e
            );
        }
    }
}

/// Load all .txt files from a directory into a NoteStore.
pub fn load_notes_from_dir(dir: &Path) -> NoteStore {
    let mut store = NoteStore::new();

    if !dir.exists() {
        if let Err(e) = fs::create_dir_all(dir) {
            error!("Failed to create notes directory {:?}: {}", dir, e);
            return store;
        }
        info!("Created notes directory: {:?}", dir);
    }

    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            error!("Failed to read notes directory {:?}: {}", dir, e);
            return store;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("txt") {
            continue;
        }

        if let Some(note) = load_note_from_file(&path) {
            store.add(note);
        }
    }

    store.sort_by_modified();
    info!("Loaded {} notes from {:?}", store.len(), dir);
    store
}

/// Load a single note from a .txt file (including sidecar metadata).
pub fn load_note_from_file(path: &Path) -> Option<Note> {
    let title = path.file_stem()?.to_str()?.to_string();
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to read {:?}: {}", path, e);
            return None;
        }
    };
    let (modified, created) = match fs::metadata(path) {
        Ok(meta) => {
            let mod_time = meta
                .modified()
                .unwrap_or_else(|_| std::time::SystemTime::now());
            // Use file creation time if available, fall back to modified time
            let create_time = meta.created().unwrap_or(mod_time);
            (
                DateTime::<Local>::from(mod_time),
                DateTime::<Local>::from(create_time),
            )
        }
        Err(_) => (Local::now(), Local::now()),
    };

    let mut note = Note::with_created(title, content, path.to_path_buf(), modified, created);

    // Load tags from sidecar metadata
    let meta = load_meta(path);
    if !meta.tags.is_empty() {
        note.set_tags(meta.tags);
        note.mark_tags_saved(); // Tags just loaded, not dirty
    }

    Some(note)
}

/// Save a note's content to its file.
pub fn save_note(note: &Note) -> Result<(), String> {
    fs::write(&note.path, &note.content)
        .map_err(|e| format!("Failed to save {:?}: {}", note.path, e))
}

/// Create a new note file.
pub fn create_note_file(dir: &Path, title: &str) -> Result<Note, String> {
    let sanitized = sanitize_filename(title);
    if sanitized.is_empty() {
        return Err("Invalid note title".to_string());
    }

    let path = dir.join(format!("{}.txt", sanitized));
    if path.exists() {
        return Err(format!("Note '{}' already exists", sanitized));
    }

    fs::write(&path, "").map_err(|e| format!("Failed to create note: {}", e))?;

    Ok(Note::new(sanitized, String::new(), path, Local::now()))
}

/// Delete a note's file from disk (and its sidecar metadata).
pub fn delete_note_file(note: &Note) -> Result<(), String> {
    delete_meta(&note.path);
    fs::remove_file(&note.path).map_err(|e| format!("Failed to delete {:?}: {}", note.path, e))
}

/// Rename a note's file on disk (and its sidecar metadata).
pub fn rename_note_file(note: &mut Note, dir: &Path, new_title: &str) -> Result<(), String> {
    let sanitized = sanitize_filename(new_title);
    if sanitized.is_empty() {
        return Err("Invalid note title".to_string());
    }

    let new_path = dir.join(format!("{}.txt", sanitized));
    if new_path.exists() && new_path != note.path {
        return Err(format!("Note '{}' already exists", sanitized));
    }

    let old_path = note.path.clone();
    fs::rename(&old_path, &new_path).map_err(|e| format!("Failed to rename: {}", e))?;
    rename_meta(&old_path, &new_path);

    note.path = new_path;
    note.update_title(sanitized);
    Ok(())
}

/// Remove characters that are invalid in filenames.
pub fn sanitize_filename(name: &str) -> String {
    name.chars()
        .filter(|c| !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
        .collect::<String>()
        .trim()
        .to_string()
}

/// Filesystem watcher that sends events via a channel.
pub struct FsWatcher {
    _watcher: RecommendedWatcher,
    pub receiver: mpsc::Receiver<Event>,
    pub recently_saved: HashSet<PathBuf>,
    pub last_save_times: std::collections::HashMap<PathBuf, Instant>,
}

impl FsWatcher {
    pub fn new(dir: &Path) -> Result<Self, String> {
        let (tx, rx) = mpsc::channel();

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        })
        .map_err(|e| format!("Failed to create watcher: {}", e))?;

        watcher
            .watch(dir, RecursiveMode::NonRecursive)
            .map_err(|e| format!("Failed to watch {:?}: {}", dir, e))?;

        info!("Watching {:?} for changes", dir);

        Ok(Self {
            _watcher: watcher,
            receiver: rx,
            recently_saved: HashSet::new(),
            last_save_times: std::collections::HashMap::new(),
        })
    }

    /// Mark a path as recently saved (to ignore the resulting fs event).
    pub fn mark_saved(&mut self, path: &Path) {
        self.recently_saved.insert(path.to_path_buf());
        self.last_save_times
            .insert(path.to_path_buf(), Instant::now());
    }

    /// Check if a path was recently saved by us (within 2 seconds).
    pub fn was_recently_saved(&mut self, path: &Path) -> bool {
        if let Some(time) = self.last_save_times.get(path) {
            if time.elapsed().as_secs() < 2 {
                return true;
            }
            self.recently_saved.remove(path);
            self.last_save_times.remove(path);
        }
        false
    }

    /// Drain pending events, returning relevant ones.
    pub fn drain_events(&mut self) -> Vec<FsChange> {
        let mut changes = Vec::new();

        while let Ok(event) = self.receiver.try_recv() {
            for path in &event.paths {
                if path.extension().and_then(|e| e.to_str()) != Some("txt") {
                    continue;
                }

                if self.was_recently_saved(path) {
                    continue;
                }

                let change = match event.kind {
                    EventKind::Create(_) => FsChange::Created(path.clone()),
                    EventKind::Modify(_) => FsChange::Modified(path.clone()),
                    EventKind::Remove(_) => FsChange::Removed(path.clone()),
                    _ => continue,
                };
                changes.push(change);
            }
        }

        changes
    }
}

#[derive(Debug)]
pub enum FsChange {
    Created(PathBuf),
    Modified(PathBuf),
    Removed(PathBuf),
}

/// Get the default notes directory.
pub fn default_notes_dir() -> PathBuf {
    dirs::document_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("rust-nv-notes")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("hello world"), "hello world");
        assert_eq!(sanitize_filename("a/b\\c:d"), "abcd");
        assert_eq!(sanitize_filename("  spaces  "), "spaces");
        assert_eq!(sanitize_filename("***"), "");
    }

    #[test]
    fn test_load_notes_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let store = load_notes_from_dir(dir.path());
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn test_create_and_load_note() {
        let dir = tempfile::tempdir().unwrap();
        let note = create_note_file(dir.path(), "Test Note").unwrap();
        assert_eq!(note.title, "Test Note");
        assert!(note.path.exists());

        let store = load_notes_from_dir(dir.path());
        assert_eq!(store.len(), 1);
        assert_eq!(store.get(0).unwrap().title, "Test Note");
    }

    #[test]
    fn test_save_note() {
        let dir = tempfile::tempdir().unwrap();
        let mut note = create_note_file(dir.path(), "Save Test").unwrap();
        note.update_content("Hello, world!".to_string());
        save_note(&note).unwrap();

        let loaded = load_note_from_file(&note.path).unwrap();
        assert_eq!(loaded.content, "Hello, world!");
    }

    #[test]
    fn test_rename_note_file() {
        let dir = tempfile::tempdir().unwrap();
        let mut note = create_note_file(dir.path(), "Old Name").unwrap();
        rename_note_file(&mut note, dir.path(), "New Name").unwrap();
        assert_eq!(note.title, "New Name");
        assert!(note.path.ends_with("New Name.txt"));
        assert!(note.path.exists());
    }

    #[test]
    fn test_delete_note_file() {
        let dir = tempfile::tempdir().unwrap();
        let note = create_note_file(dir.path(), "To Delete").unwrap();
        assert!(note.path.exists());
        delete_note_file(&note).unwrap();
        assert!(!note.path.exists());
    }

    #[test]
    fn test_create_duplicate_note() {
        let dir = tempfile::tempdir().unwrap();
        create_note_file(dir.path(), "Duplicate").unwrap();
        let result = create_note_file(dir.path(), "Duplicate");
        assert!(result.is_err());
    }

    #[test]
    fn test_create_note_invalid_title() {
        let dir = tempfile::tempdir().unwrap();
        let result = create_note_file(dir.path(), "***");
        assert!(result.is_err());
    }
}
