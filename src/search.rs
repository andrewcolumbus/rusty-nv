use crate::note::NoteStore;

/// Parsed search query with optional tag filters and text query.
pub struct ParsedQuery {
    pub tag_filters: Vec<String>,
    pub text_query: String,
}

/// Parse a query string, extracting `tag:name` prefixes.
/// e.g. "tag:rust tag:tutorial some text" -> tags=["rust","tutorial"], text="some text"
pub fn parse_query(query: &str) -> ParsedQuery {
    let mut tag_filters = Vec::new();
    let mut text_parts = Vec::new();

    for token in query.split_whitespace() {
        if let Some(tag) = token.strip_prefix("tag:") {
            if !tag.is_empty() {
                tag_filters.push(tag.to_lowercase());
            }
        } else {
            text_parts.push(token);
        }
    }

    ParsedQuery {
        tag_filters,
        text_query: text_parts.join(" "),
    }
}

/// Score a note against a query. Higher = better match.
/// Returns None if the note doesn't match at all.
/// `tags_lower` is the lowercase tags list for the note.
pub fn score_note(
    title_lower: &str,
    content_lower: &str,
    tags_lower: &[String],
    query_lower: &str,
) -> Option<i32> {
    let title_match = title_lower.contains(query_lower);
    let content_match = content_lower.contains(query_lower);
    let tag_exact = tags_lower.iter().any(|t| t == query_lower);
    let tag_substring = tags_lower.iter().any(|t| t.contains(query_lower));

    if !title_match && !content_match && !tag_exact && !tag_substring {
        return None;
    }

    let mut score = 0i32;

    if title_match {
        score += 100;
        // Exact title match is best
        if title_lower == query_lower {
            score += 1000;
        }
        // Title starts with query
        if title_lower.starts_with(query_lower) {
            score += 50;
        }
    }

    if content_match {
        score += 10;
    }

    // Exact tag match is very relevant
    if tag_exact {
        score += 200;
    } else if tag_substring {
        score += 50;
    }

    Some(score)
}

/// Search notes and return indices sorted by relevance (best first).
/// Supports `tag:name` prefix syntax for tag filtering.
pub fn search_notes(store: &NoteStore, query: &str) -> Vec<usize> {
    if query.is_empty() {
        return (0..store.len()).collect();
    }

    let parsed = parse_query(query);

    // If there are tag filters, pre-filter notes that have ALL specified tags
    let candidates: Vec<usize> = if parsed.tag_filters.is_empty() {
        (0..store.len()).collect()
    } else {
        (0..store.len())
            .filter(|&i| {
                let note = &store.notes[i];
                parsed
                    .tag_filters
                    .iter()
                    .all(|tf| note.tags_lower.iter().any(|t| t == tf))
            })
            .collect()
    };

    // If no text query remains, return tag-filtered candidates (all match)
    if parsed.text_query.is_empty() {
        return candidates;
    }

    let query_lower = parsed.text_query.to_lowercase();
    let mut scored: Vec<(usize, i32)> = candidates
        .into_iter()
        .filter_map(|i| {
            let note = &store.notes[i];
            score_note(
                &note.title_lower,
                &note.content_lower,
                &note.tags_lower,
                &query_lower,
            )
            .map(|score| (i, score))
        })
        .collect();

    scored.sort_by(|a, b| b.1.cmp(&a.1));
    scored.into_iter().map(|(i, _)| i).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_note_no_match() {
        assert_eq!(score_note("hello", "world", &[], "xyz"), None);
    }

    #[test]
    fn test_score_note_title_match() {
        let score = score_note("hello world", "some content", &[], "hello");
        assert!(score.is_some());
        assert!(score.unwrap() >= 100);
    }

    #[test]
    fn test_score_note_exact_title() {
        let score = score_note("hello", "content", &[], "hello");
        assert!(score.is_some());
        assert!(score.unwrap() >= 1000);
    }

    #[test]
    fn test_score_note_content_only() {
        let score = score_note("title", "hello world", &[], "hello");
        assert!(score.is_some());
        assert!(score.unwrap() >= 10);
        assert!(score.unwrap() < 100);
    }

    #[test]
    fn test_score_note_tag_exact_match() {
        let tags = vec!["rust".to_string()];
        let score = score_note("unrelated", "unrelated", &tags, "rust");
        assert!(score.is_some());
        assert!(score.unwrap() >= 200);
    }

    #[test]
    fn test_score_note_tag_substring_match() {
        let tags = vec!["rustlang".to_string()];
        let score = score_note("unrelated", "unrelated", &tags, "rust");
        assert!(score.is_some());
        assert!(score.unwrap() >= 50);
        // Should be less than exact tag match
        let exact_tags = vec!["rust".to_string()];
        let exact_score = score_note("unrelated", "unrelated", &exact_tags, "rust").unwrap();
        assert!(exact_score > score.unwrap());
    }

    #[test]
    fn test_search_notes_empty_query_returns_all() {
        let mut store = NoteStore::new();
        store.add(crate::note::Note::new(
            "A".into(),
            "a".into(),
            "a.txt".into(),
            chrono::Local::now(),
        ));
        store.add(crate::note::Note::new(
            "B".into(),
            "b".into(),
            "b.txt".into(),
            chrono::Local::now(),
        ));
        let results = search_notes(&store, "");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_search_notes_filters() {
        let mut store = NoteStore::new();
        store.add(crate::note::Note::new(
            "Rust notes".into(),
            "learning rust".into(),
            "rust.txt".into(),
            chrono::Local::now(),
        ));
        store.add(crate::note::Note::new(
            "Python notes".into(),
            "learning python".into(),
            "python.txt".into(),
            chrono::Local::now(),
        ));
        let results = search_notes(&store, "rust");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], 0);
    }

    #[test]
    fn test_search_notes_relevance_order() {
        let mut store = NoteStore::new();
        store.add(crate::note::Note::new(
            "Other".into(),
            "contains rust in body".into(),
            "other.txt".into(),
            chrono::Local::now(),
        ));
        store.add(crate::note::Note::new(
            "Rust".into(),
            "exact title".into(),
            "rust.txt".into(),
            chrono::Local::now(),
        ));
        let results = search_notes(&store, "rust");
        assert_eq!(results.len(), 2);
        // Exact title match should come first
        assert_eq!(results[0], 1);
    }

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

    #[test]
    fn test_search_notes_tag_prefix_filter() {
        let mut store = NoteStore::new();
        let mut note1 = crate::note::Note::new(
            "Rust Guide".into(),
            "learning rust".into(),
            "rust.txt".into(),
            chrono::Local::now(),
        );
        note1.add_tag("rust");
        note1.add_tag("tutorial");
        store.add(note1);

        let mut note2 = crate::note::Note::new(
            "Python Guide".into(),
            "learning python".into(),
            "python.txt".into(),
            chrono::Local::now(),
        );
        note2.add_tag("python");
        store.add(note2);

        // Filter by tag:rust should only return note1
        let results = search_notes(&store, "tag:rust");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], 0);

        // Filter by tag:rust with text query
        let results = search_notes(&store, "tag:rust guide");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], 0);

        // Filter by tag that no note has
        let results = search_notes(&store, "tag:javascript");
        assert!(results.is_empty());
    }
}
