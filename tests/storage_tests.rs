use std::fs;
use tempfile::tempdir;

#[path = "../src/note.rs"]
mod note;
#[path = "../src/storage.rs"]
mod storage;

use storage::*;

#[test]
fn test_load_empty_dir() {
    let dir = tempdir().unwrap();
    let store = load_notes_from_dir(dir.path());
    assert_eq!(store.len(), 0);
}

#[test]
fn test_load_creates_missing_dir() {
    let dir = tempdir().unwrap();
    let notes_dir = dir.path().join("nonexistent");
    let store = load_notes_from_dir(&notes_dir);
    assert_eq!(store.len(), 0);
    assert!(notes_dir.exists());
}

#[test]
fn test_load_ignores_non_txt() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("note.txt"), "hello").unwrap();
    fs::write(dir.path().join("image.png"), "binary").unwrap();
    fs::write(dir.path().join("data.json"), "{}").unwrap();

    let store = load_notes_from_dir(dir.path());
    assert_eq!(store.len(), 1);
}

#[test]
fn test_create_and_load_roundtrip() {
    let dir = tempdir().unwrap();
    let mut note = create_note_file(dir.path(), "My Note").unwrap();
    note.update_content("Some content here".into());
    save_note(&note).unwrap();

    let store = load_notes_from_dir(dir.path());
    assert_eq!(store.len(), 1);
    let loaded = store.get(0).unwrap();
    assert_eq!(loaded.title, "My Note");
    assert_eq!(loaded.content, "Some content here");
}

#[test]
fn test_rename_note() {
    let dir = tempdir().unwrap();
    let mut note = create_note_file(dir.path(), "Old").unwrap();
    rename_note_file(&mut note, dir.path(), "New").unwrap();

    assert_eq!(note.title, "New");
    assert!(!dir.path().join("Old.txt").exists());
    assert!(dir.path().join("New.txt").exists());
}

#[test]
fn test_rename_to_existing_fails() {
    let dir = tempdir().unwrap();
    create_note_file(dir.path(), "Existing").unwrap();
    let mut note = create_note_file(dir.path(), "Other").unwrap();

    let result = rename_note_file(&mut note, dir.path(), "Existing");
    assert!(result.is_err());
}

#[test]
fn test_delete_note() {
    let dir = tempdir().unwrap();
    let note = create_note_file(dir.path(), "Doomed").unwrap();
    assert!(note.path.exists());

    delete_note_file(&note).unwrap();
    assert!(!note.path.exists());
}

#[test]
fn test_sanitize_filename_basic() {
    assert_eq!(sanitize_filename("Hello World"), "Hello World");
}

#[test]
fn test_sanitize_filename_strips_invalid() {
    assert_eq!(sanitize_filename("a/b\\c:d*e?f\"g<h>i|j"), "abcdefghij");
}

#[test]
fn test_sanitize_filename_trims() {
    assert_eq!(sanitize_filename("  spaces  "), "spaces");
}

#[test]
fn test_sanitize_filename_all_invalid() {
    assert_eq!(sanitize_filename("***"), "");
}

#[test]
fn test_create_invalid_title_fails() {
    let dir = tempdir().unwrap();
    assert!(create_note_file(dir.path(), "***").is_err());
}

#[test]
fn test_create_duplicate_fails() {
    let dir = tempdir().unwrap();
    create_note_file(dir.path(), "Dup").unwrap();
    assert!(create_note_file(dir.path(), "Dup").is_err());
}

#[test]
fn test_default_notes_dir() {
    let dir = default_notes_dir();
    assert!(dir.to_str().unwrap().contains("rust-nv-notes"));
}

// --- Sidecar metadata tests ---

#[test]
fn test_meta_path() {
    let note_path = std::path::PathBuf::from("/tmp/test.txt");
    let meta = meta_path(&note_path);
    assert_eq!(meta, std::path::PathBuf::from("/tmp/test.txt.meta.json"));
}

#[test]
fn test_sidecar_roundtrip() {
    let dir = tempdir().unwrap();
    let note = create_note_file(dir.path(), "Tagged Note").unwrap();

    // Save metadata with tags
    let meta = NoteMeta {
        tags: vec!["rust".into(), "tutorial".into()],
    };
    save_meta(&note.path, &meta).unwrap();

    // Load it back
    let loaded_meta = load_meta(&note.path);
    assert_eq!(loaded_meta.tags, vec!["rust", "tutorial"]);
}

#[test]
fn test_sidecar_missing_returns_default() {
    let dir = tempdir().unwrap();
    let note = create_note_file(dir.path(), "No Meta").unwrap();

    let meta = load_meta(&note.path);
    assert!(meta.tags.is_empty());
}

#[test]
fn test_sidecar_malformed_returns_default() {
    let dir = tempdir().unwrap();
    let note = create_note_file(dir.path(), "Bad Meta").unwrap();

    // Write malformed JSON to the sidecar path
    let sidecar = meta_path(&note.path);
    fs::write(&sidecar, "this is not json").unwrap();

    let meta = load_meta(&note.path);
    assert!(meta.tags.is_empty());
}

#[test]
fn test_sidecar_partial_json_returns_default_tags() {
    let dir = tempdir().unwrap();
    let note = create_note_file(dir.path(), "Partial Meta").unwrap();

    // Write JSON without the tags field
    let sidecar = meta_path(&note.path);
    fs::write(&sidecar, "{}").unwrap();

    let meta = load_meta(&note.path);
    assert!(meta.tags.is_empty());
}

#[test]
fn test_delete_note_removes_sidecar() {
    let dir = tempdir().unwrap();
    let note = create_note_file(dir.path(), "Delete With Meta").unwrap();

    // Save sidecar
    let meta = NoteMeta {
        tags: vec!["rust".into()],
    };
    save_meta(&note.path, &meta).unwrap();

    let sidecar = meta_path(&note.path);
    assert!(sidecar.exists());

    // Delete note (should also remove sidecar)
    delete_note_file(&note).unwrap();
    assert!(!note.path.exists());
    assert!(!sidecar.exists());
}

#[test]
fn test_rename_note_moves_sidecar() {
    let dir = tempdir().unwrap();
    let mut note = create_note_file(dir.path(), "Old Name").unwrap();

    // Save sidecar
    let meta = NoteMeta {
        tags: vec!["important".into()],
    };
    save_meta(&note.path, &meta).unwrap();

    let old_sidecar = meta_path(&note.path);
    assert!(old_sidecar.exists());

    // Rename
    rename_note_file(&mut note, dir.path(), "New Name").unwrap();

    let new_sidecar = meta_path(&note.path);
    assert!(!old_sidecar.exists(), "Old sidecar should be gone");
    assert!(new_sidecar.exists(), "New sidecar should exist");

    // Verify contents preserved
    let loaded = load_meta(&note.path);
    assert_eq!(loaded.tags, vec!["important"]);
}

#[test]
fn test_load_note_reads_sidecar_tags() {
    let dir = tempdir().unwrap();
    let note = create_note_file(dir.path(), "With Tags").unwrap();

    // Save sidecar manually
    let meta = NoteMeta {
        tags: vec!["rust".into(), "guide".into()],
    };
    save_meta(&note.path, &meta).unwrap();

    // Load the note from file - should include tags
    let loaded = load_note_from_file(&note.path).unwrap();
    assert_eq!(loaded.tags, vec!["rust", "guide"]);
    assert_eq!(loaded.tags_lower, vec!["rust", "guide"]);
    assert!(!loaded.tags_dirty); // should not be dirty after loading
}

#[test]
fn test_load_note_without_sidecar_has_no_tags() {
    let dir = tempdir().unwrap();
    let note = create_note_file(dir.path(), "No Tags").unwrap();

    let loaded = load_note_from_file(&note.path).unwrap();
    assert!(loaded.tags.is_empty());
    assert!(!loaded.tags_dirty);
}

#[test]
fn test_delete_meta_when_no_sidecar() {
    let dir = tempdir().unwrap();
    let note = create_note_file(dir.path(), "No Sidecar").unwrap();

    // Should not panic when there's no sidecar to delete
    delete_meta(&note.path);
    assert!(note.path.exists()); // note file should still exist
}

#[test]
fn test_rename_meta_when_no_sidecar() {
    let dir = tempdir().unwrap();
    let note = create_note_file(dir.path(), "No Sidecar").unwrap();
    let new_path = dir.path().join("New Sidecar.txt");

    // Should not panic when there's no sidecar to rename
    rename_meta(&note.path, &new_path);
}

// --- Stream 4: Created field loading ---

#[test]
fn test_load_note_has_created_field() {
    let dir = tempdir().unwrap();
    let note = create_note_file(dir.path(), "Created Test").unwrap();

    let loaded = load_note_from_file(&note.path).unwrap();
    // created should be populated (either from file metadata or fallback)
    // It should be roughly "now" since we just created the file
    let now = chrono::Local::now();
    let diff = now.signed_duration_since(loaded.created);
    assert!(
        diff.num_seconds().abs() < 5,
        "Created time should be within 5 seconds of now, but diff was {} seconds",
        diff.num_seconds()
    );
}

#[test]
fn test_load_note_created_not_newer_than_modified() {
    let dir = tempdir().unwrap();
    let note = create_note_file(dir.path(), "Time Check").unwrap();

    let loaded = load_note_from_file(&note.path).unwrap();
    // For a newly created file, created <= modified
    assert!(loaded.created <= loaded.modified);
}

#[test]
fn test_load_note_bookmark_fields_default() {
    let dir = tempdir().unwrap();
    let note = create_note_file(dir.path(), "Bookmark Check").unwrap();

    let loaded = load_note_from_file(&note.path).unwrap();
    assert!(!loaded.bookmarked);
    assert_eq!(loaded.bookmark_slot, None);
}
