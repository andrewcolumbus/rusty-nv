use tempfile::tempdir;

#[path = "../src/note.rs"]
mod note;
#[path = "../src/storage.rs"]
mod storage;

use note::NoteStore;
use storage::create_note_file;

/// Helper: simulate navigate_to_link logic against a NoteStore.
/// Returns the index of the note navigated to (existing or newly created).
fn simulate_navigate_to_link(
    store: &mut NoteStore,
    notes_dir: &std::path::Path,
    target: &str,
) -> Option<usize> {
    let target_trimmed = target.trim();
    if target_trimmed.is_empty() {
        return None;
    }

    // Try to find an existing note by title (case-insensitive)
    if let Some(idx) = store.find_by_title(target_trimmed) {
        return Some(idx);
    }

    // Create a new note with the link target as title
    match create_note_file(notes_dir, target_trimmed) {
        Ok(note) => {
            store.add(note);
            Some(store.len() - 1)
        }
        Err(_) => None,
    }
}

// --- navigate_to_link: selects existing note ---

#[test]
fn test_navigate_to_link_selects_existing_note() {
    let dir = tempdir().unwrap();
    let mut store = NoteStore::new();

    let note = create_note_file(dir.path(), "My Note").unwrap();
    store.add(note);

    let idx = simulate_navigate_to_link(&mut store, dir.path(), "My Note");
    assert_eq!(idx, Some(0));
    assert_eq!(store.len(), 1); // no new note created
}

// --- navigate_to_link: case-insensitive match ---

#[test]
fn test_navigate_to_link_case_insensitive() {
    let dir = tempdir().unwrap();
    let mut store = NoteStore::new();

    let note = create_note_file(dir.path(), "Rust Guide").unwrap();
    store.add(note);

    // Navigate with different casing
    let idx = simulate_navigate_to_link(&mut store, dir.path(), "rust guide");
    assert_eq!(idx, Some(0));
    assert_eq!(store.len(), 1);
}

// --- navigate_to_link: creates note if not found ---

#[test]
fn test_navigate_to_link_creates_missing_note() {
    let dir = tempdir().unwrap();
    let mut store = NoteStore::new();

    let idx = simulate_navigate_to_link(&mut store, dir.path(), "New Topic");
    assert_eq!(idx, Some(0));
    assert_eq!(store.len(), 1);
    assert_eq!(store.get(0).unwrap().title, "New Topic");
    // Verify the file was created on disk
    assert!(dir.path().join("New Topic.txt").exists());
}

// --- navigate_to_link: empty target is ignored ---

#[test]
fn test_navigate_to_link_empty_target() {
    let dir = tempdir().unwrap();
    let mut store = NoteStore::new();

    let idx = simulate_navigate_to_link(&mut store, dir.path(), "");
    assert_eq!(idx, None);
    assert_eq!(store.len(), 0);
}

// --- navigate_to_link: whitespace-only target is ignored ---

#[test]
fn test_navigate_to_link_whitespace_target() {
    let dir = tempdir().unwrap();
    let mut store = NoteStore::new();

    let idx = simulate_navigate_to_link(&mut store, dir.path(), "   ");
    assert_eq!(idx, None);
    assert_eq!(store.len(), 0);
}

// --- navigate_to_link: multiple notes, selects correct one ---

#[test]
fn test_navigate_to_link_among_multiple_notes() {
    let dir = tempdir().unwrap();
    let mut store = NoteStore::new();

    store.add(create_note_file(dir.path(), "Alpha").unwrap());
    store.add(create_note_file(dir.path(), "Beta").unwrap());
    store.add(create_note_file(dir.path(), "Gamma").unwrap());

    let idx = simulate_navigate_to_link(&mut store, dir.path(), "Beta");
    assert_eq!(idx, Some(1));
    assert_eq!(store.len(), 3); // no new note created
}

// --- navigate_to_link: creates, then navigates to same note ---

#[test]
fn test_navigate_to_link_create_then_navigate() {
    let dir = tempdir().unwrap();
    let mut store = NoteStore::new();

    // First call creates
    let idx1 = simulate_navigate_to_link(&mut store, dir.path(), "New Note");
    assert_eq!(idx1, Some(0));
    assert_eq!(store.len(), 1);

    // Second call finds existing
    let idx2 = simulate_navigate_to_link(&mut store, dir.path(), "New Note");
    assert_eq!(idx2, Some(0));
    assert_eq!(store.len(), 1); // still just 1 note
}

// --- navigate_to_link: target with leading/trailing whitespace ---

#[test]
fn test_navigate_to_link_trimmed_target() {
    let dir = tempdir().unwrap();
    let mut store = NoteStore::new();

    store.add(create_note_file(dir.path(), "Padded").unwrap());

    // Navigate with extra whitespace
    let idx = simulate_navigate_to_link(&mut store, dir.path(), "  Padded  ");
    assert_eq!(idx, Some(0));
    assert_eq!(store.len(), 1);
}
