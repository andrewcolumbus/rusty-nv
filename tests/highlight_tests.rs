#[path = "../src/markup.rs"]
mod markup;

#[path = "../src/highlight.rs"]
mod highlight;

use egui::Color32;
use highlight::*;

// ═══════════════════════════════════════════════════════════════════
// find_search_matches
// ═══════════════════════════════════════════════════════════════════

#[test]
fn search_matches_empty_query_returns_empty() {
    assert!(find_search_matches("hello world", "").is_empty());
}

#[test]
fn search_matches_empty_text_returns_empty() {
    assert!(find_search_matches("", "query").is_empty());
}

#[test]
fn search_matches_no_match() {
    assert!(find_search_matches("hello", "xyz").is_empty());
}

#[test]
fn search_matches_basic_ascii() {
    let m = find_search_matches("hello world", "world");
    assert_eq!(m, vec![6..11]);
}

#[test]
fn search_matches_case_insensitive() {
    let m = find_search_matches("Hello WORLD", "hello");
    assert_eq!(m, vec![0..5]);
}

#[test]
fn search_matches_multiple_occurrences() {
    let m = find_search_matches("abcabc", "abc");
    assert_eq!(m, vec![0..3, 3..6]);
}

#[test]
fn search_matches_overlapping() {
    let m = find_search_matches("aaa", "aa");
    assert_eq!(m, vec![0..2, 1..3]);
}

#[test]
fn search_matches_unicode_accented() {
    // "café" where é is U+00E9 (2 bytes in UTF-8)
    let text = "I like caf\u{00e9}s";
    let m = find_search_matches(text, "caf\u{00e9}");
    assert_eq!(m.len(), 1);
    assert_eq!(m[0], 7..12);
}

#[test]
fn search_matches_unicode_emoji() {
    let text = "Hello \u{1f600} world";
    let m = find_search_matches(text, "world");
    assert_eq!(m.len(), 1);
    // "Hello " = 6 bytes, emoji \u{1f600} = 4 bytes, " " = 1 byte => "world" at 11..16
    assert_eq!(m[0], 11..16);
}

#[test]
fn search_matches_unicode_cjk() {
    let text = "\u{4f60}\u{597d}\u{4e16}\u{754c}"; // 你好世界
    let m = find_search_matches(text, "\u{4e16}\u{754c}");
    assert_eq!(m.len(), 1);
    // Each CJK char is 3 bytes, so 世界 starts at byte 6
    assert_eq!(m[0], 6..12);
}

// ═══════════════════════════════════════════════════════════════════
// find_wiki_links
// ═══════════════════════════════════════════════════════════════════

#[test]
fn wiki_links_none_found() {
    assert!(find_wiki_links("no links here").is_empty());
}

#[test]
fn wiki_links_single() {
    let links = find_wiki_links("see [[note]] here");
    assert_eq!(links, vec![4..12]);
}

#[test]
fn wiki_links_multiple() {
    let links = find_wiki_links("[[a]] and [[b]]");
    assert_eq!(links, vec![0..5, 10..15]);
}

#[test]
fn wiki_links_empty_brackets_rejected() {
    assert!(find_wiki_links("empty [[]] link").is_empty());
}

#[test]
fn wiki_links_unclosed() {
    assert!(find_wiki_links("unclosed [[link here").is_empty());
}

#[test]
fn wiki_links_no_newline_crossing() {
    assert!(find_wiki_links("[[link\nacross]]").is_empty());
}

#[test]
fn wiki_links_adjacent() {
    let links = find_wiki_links("[[a]][[b]]");
    assert_eq!(links, vec![0..5, 5..10]);
}

#[test]
fn wiki_links_with_spaces() {
    let links = find_wiki_links("[[My Note Title]]");
    assert_eq!(links.len(), 1);
    assert_eq!(&"[[My Note Title]]"[links[0].clone()], "[[My Note Title]]");
}

// ═══════════════════════════════════════════════════════════════════
// find_spans
// ═══════════════════════════════════════════════════════════════════

#[test]
fn spans_empty_text() {
    assert!(find_spans("", "query", None).is_empty());
}

#[test]
fn spans_no_query_single_normal() {
    let spans = find_spans("hello", "", None);
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].kind, SpanKind::Normal);
    assert_eq!(spans[0].byte_range, 0..5);
}

#[test]
fn spans_search_match_splits_text() {
    let spans = find_spans("hello world", "world", None);
    assert_eq!(spans.len(), 2);
    assert_eq!(spans[0].kind, SpanKind::Normal);
    assert_eq!(spans[1].kind, SpanKind::SearchMatch);
    assert_eq!(spans[1].byte_range, 6..11);
}

#[test]
fn spans_active_match_index() {
    let spans = find_spans("abc abc", "abc", Some(1));
    assert_eq!(spans.len(), 3);
    assert_eq!(spans[0].kind, SpanKind::SearchMatch);
    assert_eq!(spans[1].kind, SpanKind::Normal);
    assert_eq!(spans[2].kind, SpanKind::ActiveSearchMatch);
}

#[test]
fn spans_wiki_link_overrides_search_match() {
    let text = "see [[hello]] world";
    let spans = find_spans(text, "hello", None);
    let wiki_count = spans
        .iter()
        .filter(|s| s.kind == SpanKind::WikiLink)
        .count();
    let search_count = spans
        .iter()
        .filter(|s| s.kind == SpanKind::SearchMatch)
        .count();
    assert_eq!(wiki_count, 1);
    assert_eq!(search_count, 0);
}

#[test]
fn spans_full_byte_coverage() {
    let text = "hello [[link]] world";
    let spans = find_spans(text, "world", None);
    let total: usize = spans
        .iter()
        .map(|s| s.byte_range.end - s.byte_range.start)
        .sum();
    assert_eq!(total, text.len());
    for i in 1..spans.len() {
        assert_eq!(
            spans[i].byte_range.start,
            spans[i - 1].byte_range.end,
            "Gap between spans at index {}",
            i
        );
    }
}

#[test]
fn spans_unicode_full_coverage() {
    let text = "caf\u{00e9} [[link]] \u{1f600}";
    let spans = find_spans(text, "link", Some(0));
    let total: usize = spans
        .iter()
        .map(|s| s.byte_range.end - s.byte_range.start)
        .sum();
    assert_eq!(total, text.len());
}

// ═══════════════════════════════════════════════════════════════════
// build_layout_job
// ═══════════════════════════════════════════════════════════════════

fn test_colors() -> HighlightColors {
    HighlightColors {
        default_text: Color32::WHITE,
        search_bg: Color32::YELLOW,
        active_search_bg: Color32::from_rgba_premultiplied(255, 165, 0, 153),
        link_color: Color32::LIGHT_BLUE,
    }
}

#[test]
fn layout_job_empty() {
    let spans = find_spans("", "", None);
    let font = egui::FontId::monospace(14.0);
    let job = build_layout_job("", &spans, font, &test_colors(), 400.0);
    assert!(job.sections.is_empty());
}

#[test]
fn layout_job_plain_text() {
    let text = "plain text here";
    let spans = find_spans(text, "", None);
    let font = egui::FontId::monospace(14.0);
    let job = build_layout_job(text, &spans, font, &test_colors(), 400.0);
    assert_eq!(job.sections.len(), 1);
    assert_eq!(job.sections[0].byte_range, 0..text.len());
}

#[test]
fn layout_job_byte_coverage_with_unicode() {
    let text = "Hello \u{1f600} world [[link]]";
    let spans = find_spans(text, "world", Some(0));
    let font = egui::FontId::monospace(14.0);
    let job = build_layout_job(text, &spans, font, &test_colors(), 400.0);

    assert_eq!(job.text, text);
    let total: usize = job
        .sections
        .iter()
        .map(|s| s.byte_range.end - s.byte_range.start)
        .sum();
    assert_eq!(total, text.len());
    for i in 1..job.sections.len() {
        assert_eq!(
            job.sections[i].byte_range.start,
            job.sections[i - 1].byte_range.end
        );
    }
}

#[test]
fn layout_job_search_bg_color_set() {
    let text = "find me";
    let spans = find_spans(text, "find", None);
    let font = egui::FontId::monospace(14.0);
    let colors = test_colors();
    let job = build_layout_job(text, &spans, font, &colors, 400.0);
    // First section should have search_bg background
    assert_eq!(job.sections[0].format.background, colors.search_bg);
}

#[test]
fn layout_job_active_bg_color_set() {
    let text = "find find";
    let spans = find_spans(text, "find", Some(1));
    let font = egui::FontId::monospace(14.0);
    let colors = test_colors();
    let job = build_layout_job(text, &spans, font, &colors, 400.0);
    // Third section (second match) should have active_search_bg
    assert_eq!(job.sections[2].format.background, colors.active_search_bg);
}

#[test]
fn layout_job_link_color_set() {
    let text = "see [[link]] end";
    let spans = find_spans(text, "", None);
    let font = egui::FontId::monospace(14.0);
    let colors = test_colors();
    let job = build_layout_job(text, &spans, font, &colors, 400.0);
    // Find the wiki link section
    let link_section = job
        .sections
        .iter()
        .find(|s| s.format.color == colors.link_color);
    assert!(link_section.is_some());
}

// ═══════════════════════════════════════════════════════════════════
// byte_offset_to_char_offset / char_offset_to_byte_offset
// ═══════════════════════════════════════════════════════════════════

#[test]
fn byte_char_ascii_roundtrip() {
    let text = "hello";
    for i in 0..=text.len() {
        if text.is_char_boundary(i) {
            let char_off = byte_offset_to_char_offset(text, i);
            let back = char_offset_to_byte_offset(text, char_off);
            assert_eq!(back, i);
        }
    }
}

#[test]
fn byte_char_unicode_roundtrip() {
    let text = "Hello \u{1f600} caf\u{00e9} world";
    for (byte_idx, _) in text.char_indices() {
        let char_off = byte_offset_to_char_offset(text, byte_idx);
        let back = char_offset_to_byte_offset(text, char_off);
        assert_eq!(back, byte_idx, "roundtrip failed for byte_idx={}", byte_idx);
    }
}

#[test]
fn char_to_byte_beyond_end() {
    assert_eq!(char_offset_to_byte_offset("abc", 100), 3);
}

#[test]
fn byte_to_char_emoji() {
    let text = "a\u{1f600}b";
    assert_eq!(byte_offset_to_char_offset(text, 0), 0); // 'a'
    assert_eq!(byte_offset_to_char_offset(text, 1), 1); // emoji start
    assert_eq!(byte_offset_to_char_offset(text, 5), 2); // 'b'
    assert_eq!(byte_offset_to_char_offset(text, 6), 3); // end
}

#[test]
fn char_to_byte_emoji() {
    let text = "a\u{1f600}b";
    assert_eq!(char_offset_to_byte_offset(text, 0), 0);
    assert_eq!(char_offset_to_byte_offset(text, 1), 1);
    assert_eq!(char_offset_to_byte_offset(text, 2), 5);
    assert_eq!(char_offset_to_byte_offset(text, 3), 6);
}

#[test]
fn byte_to_char_multibyte_accented() {
    let text = "caf\u{00e9}"; // 5 bytes, 4 chars
    assert_eq!(byte_offset_to_char_offset(text, 3), 3);
    assert_eq!(byte_offset_to_char_offset(text, 5), 4);
}

// ═══════════════════════════════════════════════════════════════════
// detect_link_autocomplete_context
// ═══════════════════════════════════════════════════════════════════

#[test]
fn autocomplete_no_brackets() {
    assert!(detect_link_autocomplete_context("hello world", 5).is_none());
}

#[test]
fn autocomplete_inside_open_bracket() {
    let text = "see [[not";
    let result = detect_link_autocomplete_context(text, 9);
    assert_eq!(result, Some(("not".to_string(), 6)));
}

#[test]
fn autocomplete_after_closed_bracket() {
    assert!(detect_link_autocomplete_context("see [[note]] more", 17).is_none());
}

#[test]
fn autocomplete_empty_partial() {
    let text = "see [[";
    let result = detect_link_autocomplete_context(text, 6);
    assert_eq!(result, Some(("".to_string(), 6)));
}

#[test]
fn autocomplete_newline_breaks() {
    assert!(detect_link_autocomplete_context("see [[\nnote", 11).is_none());
}

#[test]
fn autocomplete_unicode_partial() {
    let text = "see [[caf\u{00e9}";
    let cursor = text.chars().count();
    let result = detect_link_autocomplete_context(text, cursor);
    assert_eq!(result, Some(("caf\u{00e9}".to_string(), 6)));
}
