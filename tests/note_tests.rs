use chrono::{Local, TimeZone};
use std::path::PathBuf;

// We need to reference the crate
// Since these are integration tests, we import the public API
#[path = "../src/note.rs"]
mod note;

use note::{Note, NoteStore, SortField};

#[test]
fn test_note_creation() {
    let note = Note::new(
        "Test Note".into(),
        "Hello, world!".into(),
        PathBuf::from("test.txt"),
        Local::now(),
    );
    assert_eq!(note.title, "Test Note");
    assert_eq!(note.content, "Hello, world!");
    assert_eq!(note.title_lower, "test note");
    assert_eq!(note.content_lower, "hello, world!");
    assert!(!note.dirty);
    assert!(note.tags.is_empty());
    assert!(note.tags_lower.is_empty());
    assert!(!note.tags_dirty);
}

#[test]
fn test_note_update_content() {
    let mut note = Note::new(
        "Test".into(),
        "old".into(),
        PathBuf::from("test.txt"),
        Local::now(),
    );
    note.update_content("new content".into());
    assert_eq!(note.content, "new content");
    assert_eq!(note.content_lower, "new content");
    assert!(note.dirty);
}

#[test]
fn test_note_update_content_no_change() {
    let mut note = Note::new(
        "Test".into(),
        "same".into(),
        PathBuf::from("test.txt"),
        Local::now(),
    );
    note.update_content("same".into());
    assert!(!note.dirty);
}

#[test]
fn test_note_mark_saved() {
    let mut note = Note::new(
        "Test".into(),
        "old".into(),
        PathBuf::from("test.txt"),
        Local::now(),
    );
    note.update_content("new".into());
    assert!(note.dirty);
    note.mark_saved();
    assert!(!note.dirty);
}

#[test]
fn test_store_add_and_find() {
    let mut store = NoteStore::new();
    store.add(Note::new(
        "Alpha".into(),
        "a".into(),
        PathBuf::from("a.txt"),
        Local::now(),
    ));
    store.add(Note::new(
        "Beta".into(),
        "b".into(),
        PathBuf::from("b.txt"),
        Local::now(),
    ));

    assert_eq!(store.len(), 2);
    assert_eq!(store.find_by_title("alpha"), Some(0));
    assert_eq!(store.find_by_title("Beta"), Some(1));
    assert_eq!(store.find_by_title("gamma"), None);
}

#[test]
fn test_store_remove() {
    let mut store = NoteStore::new();
    store.add(Note::new(
        "A".into(),
        "a".into(),
        PathBuf::from("a.txt"),
        Local::now(),
    ));
    store.add(Note::new(
        "B".into(),
        "b".into(),
        PathBuf::from("b.txt"),
        Local::now(),
    ));

    let removed = store.remove(0);
    assert_eq!(removed.title, "A");
    assert_eq!(store.len(), 1);
    assert_eq!(store.find_by_title("B"), Some(0));
}

#[test]
fn test_store_empty() {
    let store = NoteStore::new();
    assert!(store.is_empty());
    assert_eq!(store.len(), 0);
}

// --- Tag CRUD tests ---

#[test]
fn test_add_tag() {
    let mut note = Note::new(
        "Test".into(),
        "content".into(),
        PathBuf::from("test.txt"),
        Local::now(),
    );
    note.add_tag("rust");
    assert_eq!(note.tags, vec!["rust"]);
    assert_eq!(note.tags_lower, vec!["rust"]);
    assert!(note.tags_dirty);
}

#[test]
fn test_add_tag_trims_whitespace() {
    let mut note = Note::new(
        "Test".into(),
        "content".into(),
        PathBuf::from("test.txt"),
        Local::now(),
    );
    note.add_tag("  rust  ");
    assert_eq!(note.tags, vec!["rust"]);
}

#[test]
fn test_add_tag_empty_ignored() {
    let mut note = Note::new(
        "Test".into(),
        "content".into(),
        PathBuf::from("test.txt"),
        Local::now(),
    );
    note.add_tag("");
    note.add_tag("   ");
    assert!(note.tags.is_empty());
    assert!(!note.tags_dirty);
}

#[test]
fn test_add_tag_duplicate_prevention() {
    let mut note = Note::new(
        "Test".into(),
        "content".into(),
        PathBuf::from("test.txt"),
        Local::now(),
    );
    note.add_tag("Rust");
    note.add_tag("rust"); // duplicate (case-insensitive)
    note.add_tag("RUST"); // duplicate (case-insensitive)
    assert_eq!(note.tags.len(), 1);
    assert_eq!(note.tags[0], "Rust"); // keeps original casing
}

#[test]
fn test_remove_tag() {
    let mut note = Note::new(
        "Test".into(),
        "content".into(),
        PathBuf::from("test.txt"),
        Local::now(),
    );
    note.add_tag("rust");
    note.add_tag("tutorial");
    note.mark_tags_saved();

    note.remove_tag("rust");
    assert_eq!(note.tags, vec!["tutorial"]);
    assert_eq!(note.tags_lower, vec!["tutorial"]);
    assert!(note.tags_dirty);
}

#[test]
fn test_remove_tag_case_insensitive() {
    let mut note = Note::new(
        "Test".into(),
        "content".into(),
        PathBuf::from("test.txt"),
        Local::now(),
    );
    note.add_tag("Rust");
    note.remove_tag("RUST"); // should match case-insensitively
    assert!(note.tags.is_empty());
}

#[test]
fn test_remove_tag_nonexistent() {
    let mut note = Note::new(
        "Test".into(),
        "content".into(),
        PathBuf::from("test.txt"),
        Local::now(),
    );
    note.add_tag("rust");
    note.mark_tags_saved();

    note.remove_tag("python"); // doesn't exist
    assert_eq!(note.tags, vec!["rust"]);
    // tags_dirty should not change since nothing was removed
    assert!(!note.tags_dirty);
}

#[test]
fn test_set_tags() {
    let mut note = Note::new(
        "Test".into(),
        "content".into(),
        PathBuf::from("test.txt"),
        Local::now(),
    );
    note.set_tags(vec!["Rust".into(), "Tutorial".into()]);
    assert_eq!(note.tags, vec!["Rust", "Tutorial"]);
    assert_eq!(note.tags_lower, vec!["rust", "tutorial"]);
    assert!(note.tags_dirty);
}

#[test]
fn test_mark_tags_saved() {
    let mut note = Note::new(
        "Test".into(),
        "content".into(),
        PathBuf::from("test.txt"),
        Local::now(),
    );
    note.add_tag("rust");
    assert!(note.tags_dirty);
    note.mark_tags_saved();
    assert!(!note.tags_dirty);
}

// --- Tag index tests ---

#[test]
fn test_tag_index_built_on_add() {
    let mut store = NoteStore::new();
    let mut note = Note::new(
        "Test".into(),
        "content".into(),
        PathBuf::from("test.txt"),
        Local::now(),
    );
    note.add_tag("rust");
    note.add_tag("tutorial");
    store.add(note);

    let all_tags = store.all_tags();
    assert_eq!(all_tags.len(), 2);
    assert!(all_tags.contains(&"rust"));
    assert!(all_tags.contains(&"tutorial"));
}

#[test]
fn test_tag_index_rebuild_after_remove() {
    let mut store = NoteStore::new();
    let mut note1 = Note::new("A".into(), "a".into(), PathBuf::from("a.txt"), Local::now());
    note1.add_tag("rust");
    store.add(note1);

    let mut note2 = Note::new("B".into(), "b".into(), PathBuf::from("b.txt"), Local::now());
    note2.add_tag("python");
    store.add(note2);

    store.remove(0); // remove note with "rust" tag
    let all_tags = store.all_tags();
    assert_eq!(all_tags, vec!["python"]);
}

#[test]
fn test_all_tags_deduplicates() {
    let mut store = NoteStore::new();
    let mut note1 = Note::new("A".into(), "a".into(), PathBuf::from("a.txt"), Local::now());
    note1.add_tag("Rust");
    store.add(note1);

    let mut note2 = Note::new("B".into(), "b".into(), PathBuf::from("b.txt"), Local::now());
    note2.add_tag("rust"); // same tag, different case
    store.add(note2);

    let all_tags = store.all_tags();
    assert_eq!(all_tags.len(), 1); // should be deduplicated
}

#[test]
fn test_all_tags_sorted_alphabetically() {
    let mut store = NoteStore::new();
    let mut note = Note::new(
        "Test".into(),
        "content".into(),
        PathBuf::from("test.txt"),
        Local::now(),
    );
    note.add_tag("zebra");
    note.add_tag("apple");
    note.add_tag("mango");
    store.add(note);

    let all_tags = store.all_tags();
    assert_eq!(all_tags, vec!["apple", "mango", "zebra"]);
}

// --- Stream 4: Created field, bookmarks, sorting ---

#[test]
fn test_note_with_created_field() {
    let modified = Local::now();
    let created = Local.with_ymd_and_hms(2024, 1, 15, 10, 30, 0).unwrap();
    let note = Note::with_created(
        "Test".into(),
        "content".into(),
        PathBuf::from("test.txt"),
        modified,
        created,
    );
    assert_eq!(note.created, created);
    assert_eq!(note.modified, modified);
    assert!(!note.bookmarked);
    assert!(note.bookmark_slot.is_none());
}

#[test]
fn test_note_new_sets_created_to_modified() {
    let now = Local::now();
    let note = Note::new("Test".into(), "content".into(), PathBuf::from("t.txt"), now);
    // When using Note::new (no explicit created), created == modified
    assert_eq!(note.created, note.modified);
}

#[test]
fn test_bookmark_fields_default() {
    let note = Note::new(
        "Test".into(),
        "content".into(),
        PathBuf::from("test.txt"),
        Local::now(),
    );
    assert!(!note.bookmarked);
    assert_eq!(note.bookmark_slot, None);
}

#[test]
fn test_bookmark_toggle_on() {
    let mut note = Note::new(
        "Test".into(),
        "content".into(),
        PathBuf::from("test.txt"),
        Local::now(),
    );
    note.bookmarked = true;
    note.bookmark_slot = Some(1);
    assert!(note.bookmarked);
    assert_eq!(note.bookmark_slot, Some(1));
}

#[test]
fn test_bookmark_toggle_off() {
    let mut note = Note::new(
        "Test".into(),
        "content".into(),
        PathBuf::from("test.txt"),
        Local::now(),
    );
    note.bookmarked = true;
    note.bookmark_slot = Some(3);

    // Toggle off
    note.bookmarked = false;
    note.bookmark_slot = None;
    assert!(!note.bookmarked);
    assert_eq!(note.bookmark_slot, None);
}

#[test]
fn test_bookmark_slot_assignment_finds_free_slot() {
    let mut store = NoteStore::new();

    // Note 0 has slot 1
    let mut note0 = Note::new("A".into(), "a".into(), PathBuf::from("a.txt"), Local::now());
    note0.bookmarked = true;
    note0.bookmark_slot = Some(1);
    store.add(note0);

    // Note 1 has slot 2
    let mut note1 = Note::new("B".into(), "b".into(), PathBuf::from("b.txt"), Local::now());
    note1.bookmarked = true;
    note1.bookmark_slot = Some(2);
    store.add(note1);

    // Note 2 is not bookmarked
    let note2 = Note::new("C".into(), "c".into(), PathBuf::from("c.txt"), Local::now());
    store.add(note2);

    // Find next free slot
    let used_slots: std::collections::HashSet<u8> =
        store.notes.iter().filter_map(|n| n.bookmark_slot).collect();
    let free_slot = (1u8..=9).find(|s| !used_slots.contains(s));
    assert_eq!(free_slot, Some(3));
}

#[test]
fn test_bookmark_slot_all_slots_full() {
    let mut store = NoteStore::new();

    // Fill all 9 slots
    for i in 1u8..=9 {
        let mut note = Note::new(
            format!("Note {}", i),
            "content".into(),
            PathBuf::from(format!("{}.txt", i)),
            Local::now(),
        );
        note.bookmarked = true;
        note.bookmark_slot = Some(i);
        store.add(note);
    }

    let used_slots: std::collections::HashSet<u8> =
        store.notes.iter().filter_map(|n| n.bookmark_slot).collect();
    let free_slot = (1u8..=9).find(|s| !used_slots.contains(s));
    assert_eq!(free_slot, None);
}

#[test]
fn test_sort_field_default() {
    let sf = SortField::default();
    assert_eq!(sf, SortField::DateModified);
}

#[test]
fn test_sort_by_title_ascending() {
    let mut store = NoteStore::new();
    store.add(Note::new(
        "Zebra".into(),
        "z".into(),
        PathBuf::from("z.txt"),
        Local::now(),
    ));
    store.add(Note::new(
        "Apple".into(),
        "a".into(),
        PathBuf::from("a.txt"),
        Local::now(),
    ));
    store.add(Note::new(
        "Mango".into(),
        "m".into(),
        PathBuf::from("m.txt"),
        Local::now(),
    ));

    let mut indices: Vec<usize> = (0..store.len()).collect();
    indices.sort_by(|&a, &b| {
        let na = store.get(a).unwrap();
        let nb = store.get(b).unwrap();
        na.title_lower.cmp(&nb.title_lower)
    });

    assert_eq!(indices, vec![1, 2, 0]); // Apple, Mango, Zebra
}

#[test]
fn test_sort_by_title_descending() {
    let mut store = NoteStore::new();
    store.add(Note::new(
        "Zebra".into(),
        "z".into(),
        PathBuf::from("z.txt"),
        Local::now(),
    ));
    store.add(Note::new(
        "Apple".into(),
        "a".into(),
        PathBuf::from("a.txt"),
        Local::now(),
    ));
    store.add(Note::new(
        "Mango".into(),
        "m".into(),
        PathBuf::from("m.txt"),
        Local::now(),
    ));

    let mut indices: Vec<usize> = (0..store.len()).collect();
    indices.sort_by(|&a, &b| {
        let na = store.get(a).unwrap();
        let nb = store.get(b).unwrap();
        nb.title_lower.cmp(&na.title_lower) // reversed for descending
    });

    assert_eq!(indices, vec![0, 2, 1]); // Zebra, Mango, Apple
}

#[test]
fn test_sort_by_date_modified() {
    let t1 = Local.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
    let t2 = Local.with_ymd_and_hms(2024, 6, 15, 0, 0, 0).unwrap();
    let t3 = Local.with_ymd_and_hms(2024, 12, 31, 0, 0, 0).unwrap();

    let mut store = NoteStore::new();
    store.add(Note::new(
        "B".into(),
        "b".into(),
        PathBuf::from("b.txt"),
        t2,
    ));
    store.add(Note::new(
        "A".into(),
        "a".into(),
        PathBuf::from("a.txt"),
        t1,
    ));
    store.add(Note::new(
        "C".into(),
        "c".into(),
        PathBuf::from("c.txt"),
        t3,
    ));

    // Sort ascending by modified
    let mut indices: Vec<usize> = (0..store.len()).collect();
    indices.sort_by(|&a, &b| {
        let na = store.get(a).unwrap();
        let nb = store.get(b).unwrap();
        na.modified.cmp(&nb.modified)
    });

    assert_eq!(indices, vec![1, 0, 2]); // t1(A), t2(B), t3(C)
}

#[test]
fn test_sort_by_date_created() {
    let t1 = Local.with_ymd_and_hms(2023, 3, 10, 0, 0, 0).unwrap();
    let t2 = Local.with_ymd_and_hms(2024, 7, 20, 0, 0, 0).unwrap();
    let t3 = Local.with_ymd_and_hms(2022, 1, 5, 0, 0, 0).unwrap();

    let mod_time = Local::now();

    let mut store = NoteStore::new();
    store.add(Note::with_created(
        "B".into(),
        "b".into(),
        PathBuf::from("b.txt"),
        mod_time,
        t1,
    ));
    store.add(Note::with_created(
        "A".into(),
        "a".into(),
        PathBuf::from("a.txt"),
        mod_time,
        t2,
    ));
    store.add(Note::with_created(
        "C".into(),
        "c".into(),
        PathBuf::from("c.txt"),
        mod_time,
        t3,
    ));

    // Sort descending by created
    let mut indices: Vec<usize> = (0..store.len()).collect();
    indices.sort_by(|&a, &b| {
        let na = store.get(a).unwrap();
        let nb = store.get(b).unwrap();
        nb.created.cmp(&na.created) // reversed for descending
    });

    assert_eq!(indices, vec![1, 0, 2]); // t2(A), t1(B), t3(C) - newest first
}

#[test]
fn test_goto_bookmark_finds_correct_note() {
    let mut store = NoteStore::new();

    let mut note0 = Note::new(
        "Alpha".into(),
        "a".into(),
        PathBuf::from("a.txt"),
        Local::now(),
    );
    note0.bookmarked = true;
    note0.bookmark_slot = Some(5);
    store.add(note0);

    let note1 = Note::new(
        "Beta".into(),
        "b".into(),
        PathBuf::from("b.txt"),
        Local::now(),
    );
    store.add(note1);

    let mut note2 = Note::new(
        "Gamma".into(),
        "g".into(),
        PathBuf::from("g.txt"),
        Local::now(),
    );
    note2.bookmarked = true;
    note2.bookmark_slot = Some(3);
    store.add(note2);

    // Find note with bookmark slot 5
    let idx = store.notes.iter().position(|n| n.bookmark_slot == Some(5));
    assert_eq!(idx, Some(0));

    // Find note with bookmark slot 3
    let idx = store.notes.iter().position(|n| n.bookmark_slot == Some(3));
    assert_eq!(idx, Some(2));

    // Find note with bookmark slot 1 (doesn't exist)
    let idx = store.notes.iter().position(|n| n.bookmark_slot == Some(1));
    assert_eq!(idx, None);
}

#[test]
fn test_tab_autocomplete_logic() {
    let mut store = NoteStore::new();
    store.add(Note::new(
        "Rust Guide".into(),
        "r".into(),
        PathBuf::from("r.txt"),
        Local::now(),
    ));
    store.add(Note::new(
        "Python Guide".into(),
        "p".into(),
        PathBuf::from("p.txt"),
        Local::now(),
    ));
    store.add(Note::new(
        "Rust Tutorial".into(),
        "t".into(),
        PathBuf::from("t.txt"),
        Local::now(),
    ));

    let query_lower = "rust".to_lowercase();
    let filtered: Vec<usize> = (0..store.len()).collect();

    // Find first note whose title starts with the query
    let match_idx = filtered.iter().find(|&&idx| {
        store
            .get(idx)
            .map(|n| n.title_lower.starts_with(&query_lower))
            .unwrap_or(false)
    });

    assert_eq!(match_idx, Some(&0)); // "Rust Guide" starts with "rust"
    let note = store.get(*match_idx.unwrap()).unwrap();
    assert_eq!(note.title, "Rust Guide");
}

#[test]
fn test_tab_autocomplete_no_match() {
    let mut store = NoteStore::new();
    store.add(Note::new(
        "Alpha".into(),
        "a".into(),
        PathBuf::from("a.txt"),
        Local::now(),
    ));
    store.add(Note::new(
        "Beta".into(),
        "b".into(),
        PathBuf::from("b.txt"),
        Local::now(),
    ));

    let query_lower = "xyz".to_lowercase();
    let filtered: Vec<usize> = (0..store.len()).collect();

    let match_idx = filtered.iter().find(|&&idx| {
        store
            .get(idx)
            .map(|n| n.title_lower.starts_with(&query_lower))
            .unwrap_or(false)
    });

    assert_eq!(match_idx, None);
}

#[test]
fn test_tab_autocomplete_case_insensitive() {
    let mut store = NoteStore::new();
    store.add(Note::new(
        "MyNotes".into(),
        "m".into(),
        PathBuf::from("m.txt"),
        Local::now(),
    ));

    let query_lower = "mynotes".to_lowercase();
    let filtered: Vec<usize> = (0..store.len()).collect();

    let match_idx = filtered.iter().find(|&&idx| {
        store
            .get(idx)
            .map(|n| n.title_lower.starts_with(&query_lower))
            .unwrap_or(false)
    });

    assert_eq!(match_idx, Some(&0));
}
