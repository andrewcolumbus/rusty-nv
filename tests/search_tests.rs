use chrono::Local;
use std::path::PathBuf;

#[path = "../src/note.rs"]
mod note;
#[path = "../src/search.rs"]
mod search;

use note::{Note, NoteStore};
use search::{parse_query, score_note, search_notes};

fn make_note(title: &str, content: &str) -> Note {
    Note::new(
        title.into(),
        content.into(),
        PathBuf::from(format!("{}.txt", title)),
        Local::now(),
    )
}

fn make_tagged_note(title: &str, content: &str, tags: &[&str]) -> Note {
    let mut note = make_note(title, content);
    for tag in tags {
        note.add_tag(tag);
    }
    note
}

#[test]
fn test_score_no_match() {
    assert!(score_note("rust guide", "learning rust", &[], "python").is_none());
}

#[test]
fn test_score_title_match_higher_than_content() {
    let title_score = score_note("rust notes", "other content", &[], "rust").unwrap();
    let content_score = score_note("other title", "rust content", &[], "rust").unwrap();
    assert!(title_score > content_score);
}

#[test]
fn test_score_exact_title_highest() {
    let exact = score_note("rust", "content", &[], "rust").unwrap();
    let partial = score_note("rust notes", "content", &[], "rust").unwrap();
    assert!(exact > partial);
}

#[test]
fn test_score_tag_exact_match_bonus() {
    let tags = vec!["rust".to_string()];
    let score_with_tag = score_note("unrelated", "unrelated", &tags, "rust").unwrap();
    assert!(score_with_tag >= 200, "Exact tag match should give +200");
}

#[test]
fn test_score_tag_substring_match_bonus() {
    let tags = vec!["rustlang".to_string()];
    let score_with_tag = score_note("unrelated", "unrelated", &tags, "rust").unwrap();
    assert!(score_with_tag >= 50, "Tag substring match should give +50");

    // Exact tag match should be higher than substring
    let exact_tags = vec!["rust".to_string()];
    let exact_score = score_note("unrelated", "unrelated", &exact_tags, "rust").unwrap();
    assert!(exact_score > score_with_tag);
}

#[test]
fn test_score_no_tag_match() {
    let tags = vec!["python".to_string()];
    assert!(score_note("unrelated", "unrelated", &tags, "rust").is_none());
}

#[test]
fn test_search_empty_query() {
    let mut store = NoteStore::new();
    store.add(make_note("A", "aaa"));
    store.add(make_note("B", "bbb"));
    store.add(make_note("C", "ccc"));

    let results = search_notes(&store, "");
    assert_eq!(results.len(), 3);
}

#[test]
fn test_search_filters_non_matching() {
    let mut store = NoteStore::new();
    store.add(make_note("Rust Guide", "learning rust"));
    store.add(make_note("Python Guide", "learning python"));
    store.add(make_note("Go Guide", "learning go"));

    let results = search_notes(&store, "python");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0], 1);
}

#[test]
fn test_search_case_insensitive() {
    let mut store = NoteStore::new();
    store.add(make_note("RUST GUIDE", "LEARNING RUST"));

    let results = search_notes(&store, "rust");
    assert_eq!(results.len(), 1);
}

#[test]
fn test_search_ranks_title_over_content() {
    let mut store = NoteStore::new();
    store.add(make_note("Other", "rust is mentioned here"));
    store.add(make_note("Rust", "exact title match"));

    let results = search_notes(&store, "rust");
    assert_eq!(results.len(), 2);
    assert_eq!(results[0], 1); // Title match first
}

// --- tag: prefix parsing tests ---

#[test]
fn test_parse_query_no_tags() {
    let parsed = parse_query("hello world");
    assert!(parsed.tag_filters.is_empty());
    assert_eq!(parsed.text_query, "hello world");
}

#[test]
fn test_parse_query_with_tags() {
    let parsed = parse_query("tag:rust tag:tutorial some text");
    assert_eq!(parsed.tag_filters, vec!["rust", "tutorial"]);
    assert_eq!(parsed.text_query, "some text");
}

#[test]
fn test_parse_query_only_tags() {
    let parsed = parse_query("tag:rust tag:guide");
    assert_eq!(parsed.tag_filters, vec!["rust", "guide"]);
    assert_eq!(parsed.text_query, "");
}

#[test]
fn test_parse_query_empty_tag_ignored() {
    let parsed = parse_query("tag: hello");
    assert!(parsed.tag_filters.is_empty());
    assert_eq!(parsed.text_query, "hello");
}

// --- tag: prefix filtering in search_notes ---

#[test]
fn test_search_tag_prefix_filter() {
    let mut store = NoteStore::new();
    store.add(make_tagged_note(
        "Rust Guide",
        "learning rust",
        &["rust", "tutorial"],
    ));
    store.add(make_tagged_note(
        "Python Guide",
        "learning python",
        &["python"],
    ));

    // Filter by tag:rust should only return note1
    let results = search_notes(&store, "tag:rust");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0], 0);
}

#[test]
fn test_search_tag_prefix_with_text() {
    let mut store = NoteStore::new();
    store.add(make_tagged_note("Rust Guide", "learning rust", &["rust"]));
    store.add(make_tagged_note("Go Guide", "learning go", &["rust", "go"]));

    // tag:rust + "guide" => both have tag rust, both have "guide" in title
    let results = search_notes(&store, "tag:rust guide");
    assert_eq!(results.len(), 2);
}

#[test]
fn test_search_tag_prefix_no_match() {
    let mut store = NoteStore::new();
    store.add(make_tagged_note("Note", "content", &["rust"]));

    let results = search_notes(&store, "tag:javascript");
    assert!(results.is_empty());
}

#[test]
fn test_search_multiple_tag_filters_require_all() {
    let mut store = NoteStore::new();
    store.add(make_tagged_note("Note1", "content", &["rust", "tutorial"]));
    store.add(make_tagged_note("Note2", "content", &["rust"]));

    // Must have both tags
    let results = search_notes(&store, "tag:rust tag:tutorial");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0], 0);
}

#[test]
fn test_search_finds_note_by_tag_content() {
    // A note that only matches via tag (not title or content)
    let mut store = NoteStore::new();
    store.add(make_tagged_note("My Document", "some text here", &["rust"]));

    let results = search_notes(&store, "rust");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0], 0);
}
