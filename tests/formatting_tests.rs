//! Integration tests for formatting helpers.

#[path = "../src/ui/formatting.rs"]
mod formatting;

use formatting::{auto_indent, find_urls, indent_lines, outdent_lines, toggle_surrounding};

// ─── toggle_surrounding ────────────────────────────────────────────────────

#[test]
fn test_wrap_bold() {
    let (result, start, end) = toggle_surrounding("hello world", 0, 5, "**");
    assert_eq!(result, "**hello** world");
    assert_eq!(start, 2);
    assert_eq!(end, 7);
}

#[test]
fn test_unwrap_bold() {
    let (result, start, end) = toggle_surrounding("**hello** world", 2, 7, "**");
    assert_eq!(result, "hello world");
    assert_eq!(start, 0);
    assert_eq!(end, 5);
}

#[test]
fn test_wrap_italic_single() {
    let (result, start, end) = toggle_surrounding("some text here", 5, 9, "*");
    assert_eq!(result, "some *text* here");
    assert_eq!(start, 6);
    assert_eq!(end, 10);
}

#[test]
fn test_unwrap_italic_single() {
    let (result, start, end) = toggle_surrounding("some *text* here", 6, 10, "*");
    assert_eq!(result, "some text here");
    assert_eq!(start, 5);
    assert_eq!(end, 9);
}

#[test]
fn test_wrap_code() {
    let (result, _, _) = toggle_surrounding("function name", 9, 13, "`");
    assert_eq!(result, "function `name`");
}

#[test]
fn test_wrap_strikethrough() {
    let (result, _, _) = toggle_surrounding("old text", 0, 3, "~~");
    assert_eq!(result, "~~old~~ text");
}

#[test]
fn test_cursor_no_selection() {
    let (result, start, end) = toggle_surrounding("hello", 3, 3, "**");
    assert_eq!(result, "hel****lo");
    assert_eq!(start, 5);
    assert_eq!(end, 5);
}

#[test]
fn test_wrap_entire_string() {
    let (result, _, _) = toggle_surrounding("abc", 0, 3, "**");
    assert_eq!(result, "**abc**");
}

#[test]
fn test_wrap_empty_string() {
    let (result, start, end) = toggle_surrounding("", 0, 0, "*");
    assert_eq!(result, "**");
    assert_eq!(start, 1);
    assert_eq!(end, 1);
}

#[test]
fn test_reversed_selection() {
    // end < start should be handled by sorting
    let (result, _, _) = toggle_surrounding("hello world", 5, 0, "**");
    assert_eq!(result, "**hello** world");
}

#[test]
fn test_out_of_bounds_clamped() {
    let (result, _, _) = toggle_surrounding("hi", 0, 100, "**");
    assert_eq!(result, "**hi**");
}

#[test]
fn test_unicode_wrap() {
    let text = "\u{4e16}\u{754c}hello";
    let (result, start, end) = toggle_surrounding(text, 0, 2, "**");
    assert_eq!(result, "**\u{4e16}\u{754c}**hello");
    assert_eq!(start, 2);
    assert_eq!(end, 4);
}

// ─── indent_lines ──────────────────────────────────────────────────────────

#[test]
fn test_indent_single() {
    let (result, _, _) = indent_lines("hello", 0, 5);
    assert_eq!(result, "    hello");
}

#[test]
fn test_indent_all_lines() {
    let (result, _, _) = indent_lines("a\nb\nc", 0, 5);
    assert_eq!(result, "    a\n    b\n    c");
}

#[test]
fn test_indent_middle_line_only() {
    let (result, _, _) = indent_lines("a\nb\nc", 2, 3);
    assert_eq!(result, "a\n    b\nc");
}

#[test]
fn test_indent_empty_line() {
    let (result, _, _) = indent_lines("\n", 0, 1);
    assert_eq!(result, "    \n");
}

#[test]
fn test_indent_preserves_existing() {
    let (result, _, _) = indent_lines("    already", 0, 11);
    assert_eq!(result, "        already");
}

// ─── outdent_lines ─────────────────────────────────────────────────────────

#[test]
fn test_outdent_four_spaces() {
    let (result, _, _) = outdent_lines("    hello", 0, 9);
    assert_eq!(result, "hello");
}

#[test]
fn test_outdent_less_than_four() {
    let (result, _, _) = outdent_lines("  hello", 0, 7);
    assert_eq!(result, "hello");
}

#[test]
fn test_outdent_more_than_four() {
    let (result, _, _) = outdent_lines("      hello", 0, 11);
    assert_eq!(result, "  hello");
}

#[test]
fn test_outdent_no_leading_spaces() {
    let (result, _, _) = outdent_lines("hello", 0, 5);
    assert_eq!(result, "hello");
}

#[test]
fn test_outdent_all_lines() {
    let (result, _, _) = outdent_lines("    a\n    b\n    c", 0, 17);
    assert_eq!(result, "a\nb\nc");
}

#[test]
fn test_outdent_middle_only() {
    let (result, _, _) = outdent_lines("a\n    b\nc", 2, 7);
    assert_eq!(result, "a\nb\nc");
}

#[test]
fn test_indent_then_outdent_roundtrip() {
    let original = "line1\nline2\nline3";
    let (indented, _, _) = indent_lines(original, 0, 17);
    let (result, _, _) = outdent_lines(&indented, 0, indented.chars().count());
    assert_eq!(result, original);
}

// ─── auto_indent ───────────────────────────────────────────────────────────

#[test]
fn test_auto_indent_whitespace() {
    let content = "    hello\n";
    let cursor = content.len();
    let result = auto_indent(content, cursor);
    assert!(result.is_some());
    let (new_content, new_cursor) = result.unwrap();
    assert_eq!(new_content, "    hello\n    ");
    assert_eq!(new_cursor, 14);
}

#[test]
fn test_auto_indent_dash_list() {
    let content = "- item one\n";
    let cursor = content.len();
    let result = auto_indent(content, cursor);
    assert!(result.is_some());
    let (new_content, _) = result.unwrap();
    assert_eq!(new_content, "- item one\n- ");
}

#[test]
fn test_auto_indent_asterisk_list() {
    let content = "* item one\n";
    let cursor = content.len();
    let result = auto_indent(content, cursor);
    assert!(result.is_some());
    let (new_content, _) = result.unwrap();
    assert_eq!(new_content, "* item one\n* ");
}

#[test]
fn test_auto_indent_ordered_list_increment() {
    let content = "3. third\n";
    let cursor = content.len();
    let result = auto_indent(content, cursor);
    assert!(result.is_some());
    let (new_content, _) = result.unwrap();
    assert_eq!(new_content, "3. third\n4. ");
}

#[test]
fn test_auto_indent_indented_list() {
    let content = "    - nested item\n";
    let cursor = content.len();
    let result = auto_indent(content, cursor);
    assert!(result.is_some());
    let (new_content, _) = result.unwrap();
    assert_eq!(new_content, "    - nested item\n    - ");
}

#[test]
fn test_auto_indent_no_indent_needed() {
    let content = "plain text\n";
    let cursor = content.len();
    let result = auto_indent(content, cursor);
    assert!(result.is_none());
}

#[test]
fn test_auto_indent_cursor_not_after_newline() {
    let content = "hello world";
    let result = auto_indent(content, 5);
    assert!(result.is_none());
}

#[test]
fn test_auto_indent_empty_content() {
    let result = auto_indent("", 0);
    assert!(result.is_none());
}

#[test]
fn test_auto_indent_tab_indentation() {
    let content = "\tindented\n";
    let cursor = content.len();
    let result = auto_indent(content, cursor);
    assert!(result.is_some());
    let (new_content, _) = result.unwrap();
    assert_eq!(new_content, "\tindented\n\t");
}

#[test]
fn test_auto_indent_middle_of_document() {
    let content = "first\n    second\nthird";
    // Cursor right after "second\n" — but that's a newline in the middle
    // We need to find position of the \n after "second"
    let pos = "first\n    second\n".len();
    let result = auto_indent(content, pos);
    assert!(result.is_some());
    let (new_content, _) = result.unwrap();
    assert_eq!(new_content, "first\n    second\n    third");
}

// ─── find_urls ─────────────────────────────────────────────────────────────

#[test]
fn test_find_urls_various_protocols() {
    for (text, expected) in [
        ("https://example.com", "https://example.com"),
        ("http://example.com", "http://example.com"),
        ("mailto:user@example.com", "mailto:user@example.com"),
    ] {
        let urls = find_urls(text);
        assert_eq!(urls.len(), 1);
        assert_eq!(&text[urls[0].clone()], expected);
    }
}

#[test]
fn test_find_urls_with_query() {
    let text = "https://example.com/path?key=value&other=1";
    let urls = find_urls(text);
    assert_eq!(urls.len(), 1);
    assert_eq!(&text[urls[0].clone()], text);
}

#[test]
fn test_find_urls_multiple_in_text() {
    let text = "Visit https://a.com and http://b.org today";
    let urls = find_urls(text);
    assert_eq!(urls.len(), 2);
    assert_eq!(&text[urls[0].clone()], "https://a.com");
    assert_eq!(&text[urls[1].clone()], "http://b.org");
}

#[test]
fn test_find_urls_none() {
    assert!(find_urls("No URLs here").is_empty());
    assert!(find_urls("").is_empty());
    assert!(find_urls("just text with no protocol").is_empty());
}

#[test]
fn test_find_urls_in_brackets() {
    let text = "Link: [https://example.com](info)";
    let urls = find_urls(text);
    assert_eq!(urls.len(), 1);
    assert_eq!(&text[urls[0].clone()], "https://example.com");
}

#[test]
fn test_find_urls_in_parens() {
    let text = "(see https://example.com)";
    let urls = find_urls(text);
    assert_eq!(urls.len(), 1);
    assert_eq!(&text[urls[0].clone()], "https://example.com");
}

#[test]
fn test_find_urls_at_end_of_text() {
    let text = "Visit https://example.com";
    let urls = find_urls(text);
    assert_eq!(urls.len(), 1);
    assert_eq!(&text[urls[0].clone()], "https://example.com");
}
