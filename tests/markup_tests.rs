//! Integration tests for the markup parser.

#[path = "../src/markup.rs"]
mod markup;

use markup::{parse, Style};
use std::ops::Range;

/// Helper: verify that spans cover every byte exactly once.
fn assert_full_coverage(spans: &[(Range<usize>, Style)], text_len: usize) {
    if text_len == 0 {
        assert!(spans.is_empty(), "Empty text should have no spans");
        return;
    }
    let mut covered = vec![false; text_len];
    for (range, _) in spans {
        assert!(range.start < range.end, "Empty range found: {:?}", range);
        assert!(
            range.end <= text_len,
            "Range {:?} exceeds text length {}",
            range,
            text_len
        );
        for i in range.start..range.end {
            assert!(!covered[i], "Byte {} covered more than once", i);
            covered[i] = true;
        }
    }
    for (i, c) in covered.iter().enumerate() {
        assert!(*c, "Byte {} not covered", i);
    }
}

/// Helper: verify spans are sorted and non-overlapping.
fn assert_sorted(spans: &[(Range<usize>, Style)]) {
    for w in spans.windows(2) {
        assert!(
            w[0].0.end <= w[1].0.start,
            "Spans overlap or out of order: {:?} and {:?}",
            w[0].0,
            w[1].0
        );
    }
}

// ─── Basic syntax tests ────────────────────────────────────────────────────

#[test]
fn test_empty_input() {
    let spans = parse("");
    assert!(spans.is_empty());
}

#[test]
fn test_plain_text_no_formatting() {
    let text = "Just plain text with no markdown.";
    let spans = parse(text);
    assert_full_coverage(&spans, text.len());
    assert_sorted(&spans);
    for (_, style) in &spans {
        assert_eq!(*style, Style::default());
    }
}

#[test]
fn test_bold_asterisks() {
    let text = "before **bold text** after";
    let spans = parse(text);
    assert_full_coverage(&spans, text.len());
    assert_sorted(&spans);

    let bold: Vec<_> = spans.iter().filter(|(_, s)| s.bold).collect();
    assert_eq!(bold.len(), 1);
    assert_eq!(&text[bold[0].0.clone()], "**bold text**");
}

#[test]
fn test_italic_asterisk() {
    let text = "before *italic text* after";
    let spans = parse(text);
    assert_full_coverage(&spans, text.len());
    assert_sorted(&spans);

    let italic: Vec<_> = spans.iter().filter(|(_, s)| s.italic).collect();
    assert_eq!(italic.len(), 1);
    assert_eq!(&text[italic[0].0.clone()], "*italic text*");
}

#[test]
fn test_italic_underscore() {
    let text = "before _italic text_ after";
    let spans = parse(text);
    assert_full_coverage(&spans, text.len());
    assert_sorted(&spans);

    let italic: Vec<_> = spans.iter().filter(|(_, s)| s.italic).collect();
    assert_eq!(italic.len(), 1);
    assert_eq!(&text[italic[0].0.clone()], "_italic text_");
}

#[test]
fn test_strikethrough() {
    let text = "before ~~struck out~~ after";
    let spans = parse(text);
    assert_full_coverage(&spans, text.len());
    assert_sorted(&spans);

    let strike: Vec<_> = spans.iter().filter(|(_, s)| s.strikethrough).collect();
    assert_eq!(strike.len(), 1);
    assert_eq!(&text[strike[0].0.clone()], "~~struck out~~");
}

#[test]
fn test_inline_code() {
    let text = "before `code here` after";
    let spans = parse(text);
    assert_full_coverage(&spans, text.len());
    assert_sorted(&spans);

    let code: Vec<_> = spans.iter().filter(|(_, s)| s.code).collect();
    assert_eq!(code.len(), 1);
    assert_eq!(&text[code[0].0.clone()], "`code here`");
}

// ─── Heading tests ─────────────────────────────────────────────────────────

#[test]
fn test_heading_h1_through_h6() {
    for level in 1u8..=6 {
        let prefix = "#".repeat(level as usize);
        let text = format!("{} Heading level {}", prefix, level);
        let spans = parse(&text);
        assert_full_coverage(&spans, text.len());
        for (_, style) in &spans {
            assert_eq!(
                style.heading, level,
                "Expected heading level {} for '{}'",
                level, text
            );
        }
    }
}

#[test]
fn test_heading_requires_space() {
    let text = "#NoSpace";
    let spans = parse(text);
    assert_full_coverage(&spans, text.len());
    for (_, style) in &spans {
        assert_eq!(style.heading, 0, "Should not be heading without space");
    }
}

#[test]
fn test_heading_level_7_not_valid() {
    let text = "####### Too many hashes";
    let spans = parse(text);
    assert_full_coverage(&spans, text.len());
    for (_, style) in &spans {
        assert_eq!(style.heading, 0);
    }
}

// ─── Quote tests ───────────────────────────────────────────────────────────

#[test]
fn test_quote_basic() {
    let text = "> This is a quote";
    let spans = parse(text);
    assert_full_coverage(&spans, text.len());
    for (_, style) in &spans {
        assert!(style.quote);
    }
}

#[test]
fn test_quote_requires_space() {
    let text = ">NotAQuote";
    let spans = parse(text);
    assert_full_coverage(&spans, text.len());
    for (_, style) in &spans {
        assert!(!style.quote);
    }
}

// ─── URL tests ─────────────────────────────────────────────────────────────

#[test]
fn test_url_https() {
    let text = "See https://example.com/path today";
    let spans = parse(text);
    assert_full_coverage(&spans, text.len());
    assert_sorted(&spans);

    let url: Vec<_> = spans.iter().filter(|(_, s)| s.url).collect();
    assert_eq!(url.len(), 1);
    assert_eq!(&text[url[0].0.clone()], "https://example.com/path");
}

#[test]
fn test_url_http() {
    let text = "Visit http://example.com please";
    let spans = parse(text);
    assert_full_coverage(&spans, text.len());

    let url: Vec<_> = spans.iter().filter(|(_, s)| s.url).collect();
    assert_eq!(url.len(), 1);
    assert_eq!(&text[url[0].0.clone()], "http://example.com");
}

#[test]
fn test_url_mailto() {
    let text = "Email mailto:test@example.com now";
    let spans = parse(text);
    assert_full_coverage(&spans, text.len());

    let url: Vec<_> = spans.iter().filter(|(_, s)| s.url).collect();
    assert_eq!(url.len(), 1);
    assert_eq!(&text[url[0].0.clone()], "mailto:test@example.com");
}

#[test]
fn test_url_stops_at_special_chars() {
    for (text, expected_url) in [
        ("[https://a.com]", "https://a.com"),
        ("(https://b.com)", "https://b.com"),
        ("<https://c.com>", "https://c.com"),
    ] {
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());
        let url: Vec<_> = spans.iter().filter(|(_, s)| s.url).collect();
        assert_eq!(url.len(), 1, "Failed for input: {}", text);
        assert_eq!(&text[url[0].0.clone()], expected_url);
    }
}

#[test]
fn test_multiple_urls() {
    let text = "https://a.com and https://b.org end";
    let spans = parse(text);
    assert_full_coverage(&spans, text.len());
    let url: Vec<_> = spans.iter().filter(|(_, s)| s.url).collect();
    assert_eq!(url.len(), 2);
}

// ─── Edge cases: unmatched delimiters ──────────────────────────────────────

#[test]
fn test_unmatched_bold() {
    let text = "before **unmatched after";
    let spans = parse(text);
    assert_full_coverage(&spans, text.len());
    let bold: Vec<_> = spans.iter().filter(|(_, s)| s.bold).collect();
    assert_eq!(bold.len(), 0, "Unmatched ** should not produce bold");
}

#[test]
fn test_unmatched_italic() {
    let text = "before *unmatched after";
    let spans = parse(text);
    assert_full_coverage(&spans, text.len());
    let italic: Vec<_> = spans.iter().filter(|(_, s)| s.italic).collect();
    assert_eq!(italic.len(), 0);
}

#[test]
fn test_unmatched_strikethrough() {
    let text = "before ~~unmatched after";
    let spans = parse(text);
    assert_full_coverage(&spans, text.len());
    let strike: Vec<_> = spans.iter().filter(|(_, s)| s.strikethrough).collect();
    assert_eq!(strike.len(), 0);
}

#[test]
fn test_unmatched_code() {
    let text = "before `unmatched after";
    let spans = parse(text);
    assert_full_coverage(&spans, text.len());
    let code: Vec<_> = spans.iter().filter(|(_, s)| s.code).collect();
    assert_eq!(code.len(), 0);
}

// ─── Code spans protect inner delimiters ───────────────────────────────────

#[test]
fn test_code_protects_bold() {
    let text = "`**not bold**`";
    let spans = parse(text);
    assert_full_coverage(&spans, text.len());
    let bold: Vec<_> = spans.iter().filter(|(_, s)| s.bold).collect();
    assert_eq!(bold.len(), 0, "Delimiters inside code should be ignored");
    let code: Vec<_> = spans.iter().filter(|(_, s)| s.code).collect();
    assert_eq!(code.len(), 1);
}

#[test]
fn test_code_protects_italic() {
    let text = "`*not italic*`";
    let spans = parse(text);
    assert_full_coverage(&spans, text.len());
    let italic: Vec<_> = spans.iter().filter(|(_, s)| s.italic).collect();
    assert_eq!(italic.len(), 0);
}

#[test]
fn test_code_protects_strikethrough() {
    let text = "`~~not struck~~`";
    let spans = parse(text);
    assert_full_coverage(&spans, text.len());
    let strike: Vec<_> = spans.iter().filter(|(_, s)| s.strikethrough).collect();
    assert_eq!(strike.len(), 0);
}

// ─── Formatting does not cross line boundaries ─────────────────────────────

#[test]
fn test_bold_no_cross_line() {
    let text = "**bold\nstill**";
    let spans = parse(text);
    assert_full_coverage(&spans, text.len());
    let bold: Vec<_> = spans.iter().filter(|(_, s)| s.bold).collect();
    assert_eq!(bold.len(), 0, "Bold should not cross line boundaries");
}

#[test]
fn test_code_no_cross_line() {
    let text = "`code\nnewline`";
    let spans = parse(text);
    assert_full_coverage(&spans, text.len());
    let code: Vec<_> = spans.iter().filter(|(_, s)| s.code).collect();
    assert_eq!(code.len(), 0, "Code should not cross line boundaries");
}

#[test]
fn test_italic_no_cross_line() {
    let text = "*italic\nnewline*";
    let spans = parse(text);
    assert_full_coverage(&spans, text.len());
    let italic: Vec<_> = spans.iter().filter(|(_, s)| s.italic).collect();
    assert_eq!(italic.len(), 0);
}

// ─── Nested / combined formatting ──────────────────────────────────────────

#[test]
fn test_heading_with_bold() {
    let text = "## **Bold heading**";
    let spans = parse(text);
    assert_full_coverage(&spans, text.len());
    let heading_bold: Vec<_> = spans
        .iter()
        .filter(|(_, s)| s.heading == 2 && s.bold)
        .collect();
    assert!(!heading_bold.is_empty(), "Should combine heading + bold");
}

#[test]
fn test_quote_with_italic() {
    let text = "> *quoted italic*";
    let spans = parse(text);
    assert_full_coverage(&spans, text.len());
    let quote_italic: Vec<_> = spans.iter().filter(|(_, s)| s.quote && s.italic).collect();
    assert!(!quote_italic.is_empty(), "Should combine quote + italic");
}

#[test]
fn test_multiple_formats_one_line() {
    let text = "**bold** and *italic* and `code`";
    let spans = parse(text);
    assert_full_coverage(&spans, text.len());
    assert_sorted(&spans);

    assert_eq!(spans.iter().filter(|(_, s)| s.bold).count(), 1);
    assert_eq!(spans.iter().filter(|(_, s)| s.italic).count(), 1);
    assert_eq!(spans.iter().filter(|(_, s)| s.code).count(), 1);
}

#[test]
fn test_adjacent_bold_and_italic() {
    let text = "**bold***italic*";
    let spans = parse(text);
    assert_full_coverage(&spans, text.len());
    assert_sorted(&spans);

    let bold: Vec<_> = spans.iter().filter(|(_, s)| s.bold).collect();
    assert_eq!(bold.len(), 1);
    assert_eq!(&text[bold[0].0.clone()], "**bold**");

    let italic: Vec<_> = spans.iter().filter(|(_, s)| s.italic).collect();
    assert_eq!(italic.len(), 1);
    assert_eq!(&text[italic[0].0.clone()], "*italic*");
}

// ─── Multiline documents ──────────────────────────────────────────────────

#[test]
fn test_multiline_mixed_document() {
    let text =
        "# Title\n\nRegular paragraph with **bold** and *italic*.\n\n> A quote\n\n- List item\n";
    let spans = parse(text);
    assert_full_coverage(&spans, text.len());
    assert_sorted(&spans);
}

#[test]
fn test_windows_line_endings() {
    let text = "# Heading\r\nNormal\r\n> Quote\r\n";
    let spans = parse(text);
    assert_full_coverage(&spans, text.len());
    assert_sorted(&spans);
}

// ─── Unicode handling ──────────────────────────────────────────────────────

#[test]
fn test_unicode_in_formatting() {
    let text = "**\u{4e16}\u{754c}** and *\u{0431}\u{043e}\u{043b}\u{0434}*";
    let spans = parse(text);
    assert_full_coverage(&spans, text.len());
    assert_sorted(&spans);

    let bold: Vec<_> = spans.iter().filter(|(_, s)| s.bold).collect();
    assert_eq!(bold.len(), 1);
    let italic: Vec<_> = spans.iter().filter(|(_, s)| s.italic).collect();
    assert_eq!(italic.len(), 1);
}

#[test]
fn test_emoji_text() {
    let text = "Hello \u{1f600} **bold \u{1f60d}** world";
    let spans = parse(text);
    assert_full_coverage(&spans, text.len());
    assert_sorted(&spans);
}

// ─── Empty formatting delimiters ───────────────────────────────────────────

#[test]
fn test_empty_bold_no_match() {
    let text = "**** not bold";
    let spans = parse(text);
    assert_full_coverage(&spans, text.len());
}

#[test]
fn test_empty_italic_no_match() {
    let text = "** not italic";
    let spans = parse(text);
    assert_full_coverage(&spans, text.len());
}

#[test]
fn test_single_tilde_not_strikethrough() {
    let text = "~not strike~";
    let spans = parse(text);
    assert_full_coverage(&spans, text.len());
    let strike: Vec<_> = spans.iter().filter(|(_, s)| s.strikethrough).collect();
    assert_eq!(strike.len(), 0);
}

// ─── Stress tests ──────────────────────────────────────────────────────────

#[test]
fn test_single_char() {
    let text = "x";
    let spans = parse(text);
    assert_full_coverage(&spans, text.len());
}

#[test]
fn test_only_newlines() {
    let text = "\n\n\n";
    let spans = parse(text);
    assert_full_coverage(&spans, text.len());
}

#[test]
fn test_only_delimiters() {
    for text in ["**", "***", "~~", "`", "``", "###"] {
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());
    }
}

#[test]
fn test_long_url() {
    let text =
        "Link: https://example.com/very/long/path/with/many/segments?query=1&other=2#fragment";
    let spans = parse(text);
    assert_full_coverage(&spans, text.len());
    let url: Vec<_> = spans.iter().filter(|(_, s)| s.url).collect();
    assert_eq!(url.len(), 1);
}
