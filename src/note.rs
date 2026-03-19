use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Sort field for the note list columns.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortField {
    Title,
    #[default]
    DateModified,
    DateCreated,
}

#[derive(Debug, Clone)]
pub struct Note {
    pub title: String,
    pub content: String,
    pub path: PathBuf,
    pub modified: DateTime<Local>,
    pub created: DateTime<Local>,
    pub dirty: bool,
    pub title_lower: String,
    pub content_lower: String,
    pub tags: Vec<String>,
    pub tags_lower: Vec<String>,
    pub tags_dirty: bool,
    pub bookmarked: bool,
    pub bookmark_slot: Option<u8>,
}

impl Note {
    pub fn new(title: String, content: String, path: PathBuf, modified: DateTime<Local>) -> Self {
        Self::with_created(title, content, path, modified, modified)
    }

    pub fn with_created(
        title: String,
        content: String,
        path: PathBuf,
        modified: DateTime<Local>,
        created: DateTime<Local>,
    ) -> Self {
        let title_lower = title.to_lowercase();
        let content_lower = content.to_lowercase();
        Self {
            title,
            content,
            path,
            modified,
            created,
            dirty: false,
            title_lower,
            content_lower,
            tags: Vec::new(),
            tags_lower: Vec::new(),
            tags_dirty: false,
            bookmarked: false,
            bookmark_slot: None,
        }
    }

    pub fn update_content(&mut self, new_content: String) {
        if self.content != new_content {
            self.content = new_content;
            self.content_lower = self.content.to_lowercase();
            self.dirty = true;
            self.modified = Local::now();
        }
    }

    pub fn update_title(&mut self, new_title: String) {
        self.title = new_title;
        self.title_lower = self.title.to_lowercase();
    }

    pub fn mark_saved(&mut self) {
        self.dirty = false;
    }

    /// Set the full tags list, replacing any existing tags.
    pub fn set_tags(&mut self, tags: Vec<String>) {
        self.tags = tags;
        self.tags_lower = self.tags.iter().map(|t| t.to_lowercase()).collect();
        self.tags_dirty = true;
    }

    /// Add a tag if it doesn't already exist (case-insensitive dedup).
    pub fn add_tag(&mut self, tag: &str) {
        let tag_trimmed = tag.trim().to_string();
        if tag_trimmed.is_empty() {
            return;
        }
        let tag_lower = tag_trimmed.to_lowercase();
        if self.tags_lower.contains(&tag_lower) {
            return;
        }
        self.tags.push(tag_trimmed);
        self.tags_lower.push(tag_lower);
        self.tags_dirty = true;
    }

    /// Remove a tag by exact name (case-insensitive match).
    pub fn remove_tag(&mut self, tag: &str) {
        let tag_lower = tag.to_lowercase();
        if let Some(pos) = self.tags_lower.iter().position(|t| *t == tag_lower) {
            self.tags.remove(pos);
            self.tags_lower.remove(pos);
            self.tags_dirty = true;
        }
    }

    /// Mark tags as saved (reset the tags_dirty flag).
    pub fn mark_tags_saved(&mut self) {
        self.tags_dirty = false;
    }
}

#[derive(Debug, Default)]
#[allow(clippy::len_without_is_empty)]
pub struct NoteStore {
    pub notes: Vec<Note>,
    title_index: HashMap<String, usize>,
    tag_index: HashMap<String, Vec<usize>>,
}

impl NoteStore {
    pub fn new() -> Self {
        Self {
            notes: Vec::new(),
            title_index: HashMap::new(),
            tag_index: HashMap::new(),
        }
    }

    pub fn rebuild_index(&mut self) {
        self.title_index.clear();
        for (i, note) in self.notes.iter().enumerate() {
            self.title_index.insert(note.title_lower.clone(), i);
        }
        self.rebuild_tag_index();
    }

    /// Rebuild the tag index from all notes.
    pub fn rebuild_tag_index(&mut self) {
        self.tag_index.clear();
        for (i, note) in self.notes.iter().enumerate() {
            for tag_lower in &note.tags_lower {
                self.tag_index.entry(tag_lower.clone()).or_default().push(i);
            }
        }
    }

    /// Return all unique tag names sorted alphabetically (for autocomplete).
    pub fn all_tags(&self) -> Vec<&str> {
        let mut tags: Vec<&str> = Vec::new();
        for note in &self.notes {
            for tag in &note.tags {
                let tag_lower = tag.to_lowercase();
                if !tags.iter().any(|t| t.to_lowercase() == tag_lower) {
                    tags.push(tag);
                }
            }
        }
        tags.sort_by_key(|a| a.to_lowercase());
        tags
    }

    pub fn add(&mut self, note: Note) {
        let key = note.title_lower.clone();
        let idx = self.notes.len();
        // Update tag index for the new note
        for tag_lower in &note.tags_lower {
            self.tag_index
                .entry(tag_lower.clone())
                .or_default()
                .push(idx);
        }
        self.notes.push(note);
        self.title_index.insert(key, idx);
    }

    pub fn remove(&mut self, index: usize) -> Note {
        let note = self.notes.remove(index);
        self.rebuild_index();
        note
    }

    pub fn find_by_title(&self, title: &str) -> Option<usize> {
        self.title_index.get(&title.to_lowercase()).copied()
    }

    pub fn get(&self, index: usize) -> Option<&Note> {
        self.notes.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut Note> {
        self.notes.get_mut(index)
    }

    pub fn len(&self) -> usize {
        self.notes.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.notes.is_empty()
    }

    pub fn sort_by_modified(&mut self) {
        self.notes.sort_by(|a, b| b.modified.cmp(&a.modified));
        self.rebuild_index();
    }
}
