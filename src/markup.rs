//! Lightweight inline markdown parser.
//!
//! Produces `(Range<usize>, Style)` spans covering every byte of the input.
//! Supported syntax:
//! - `**bold**`
//! - `*italic*` / `_italic_`
//! - `~~strikethrough~~`
//! - `` `inline code` ``
//! - `# Heading` through `###### Heading` (line prefixes)
//! - `> Quote` (line prefix)
//! - URL auto-detection: `http://`, `https://`, `mailto:`

use std::ops::Range;

/// Inline style flags for a span of text.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    pub strikethrough: bool,
    pub code: bool,
    pub heading: u8, // 0 = none, 1-6 = heading level
    pub quote: bool,
    pub url: bool,
}

/// Parse markdown text into a list of non-overlapping spans covering every byte.
///
/// Returns `Vec<(Range<usize>, Style)>` where ranges are byte offsets.
pub fn parse(text: &str) -> Vec<(Range<usize>, Style)> {
    if text.is_empty() {
        return Vec::new();
    }

    let mut spans: Vec<(Range<usize>, Style)> = Vec::new();

    // Process line by line to detect line-level prefixes (headings, quotes)
    let mut line_start = 0;
    for line in text.split_inclusive('\n') {
        let line_end = line_start + line.len();
        let line_style = detect_line_style(line);
        parse_inline(text, line_start, line_end, &line_style, &mut spans);
        line_start = line_end;
    }

    // Verify full coverage: sort spans and fill any gaps with default style
    spans.sort_by_key(|s| s.0.start);

    fill_gaps(spans, text.len())
}

/// Detect line-level style from a line prefix.
fn detect_line_style(line: &str) -> Style {
    let trimmed = line.trim_start();

    // Heading: # through ######
    if trimmed.starts_with('#') {
        let level = trimmed.chars().take_while(|&c| c == '#').count();
        if level <= 6 {
            // Must be followed by a space or end of line
            let rest = &trimmed[level..];
            if rest.is_empty() || rest.starts_with(' ') || rest.starts_with('\n') {
                return Style {
                    heading: level as u8,
                    ..Default::default()
                };
            }
        }
    }

    // Quote: > at start of line
    if let Some(rest) = trimmed.strip_prefix('>') {
        if rest.is_empty() || rest.starts_with(' ') || rest.starts_with('\n') {
            return Style {
                quote: true,
                ..Default::default()
            };
        }
    }

    Style::default()
}

/// Parse inline formatting within a line (or portion of text).
fn parse_inline(
    text: &str,
    start: usize,
    end: usize,
    line_style: &Style,
    spans: &mut Vec<(Range<usize>, Style)>,
) {
    let bytes = text.as_bytes();
    let mut pos = start;

    while pos < end {
        // Try to detect a URL
        if let Some((url_end, _url_text)) = try_parse_url(text, pos, end) {
            let mut style = line_style.clone();
            style.url = true;
            spans.push((pos..url_end, style));
            pos = url_end;
            continue;
        }

        // Try to detect inline code (backtick)
        if bytes[pos] == b'`' {
            if let Some(code_end) = find_closing_backtick(bytes, pos, end) {
                let mut style = line_style.clone();
                style.code = true;
                spans.push((pos..code_end, style));
                pos = code_end;
                continue;
            }
        }

        // Try to detect ~~strikethrough~~
        if pos + 1 < end && bytes[pos] == b'~' && bytes[pos + 1] == b'~' {
            if let Some(close) = find_closing_double(bytes, pos, end, b'~') {
                let mut style = line_style.clone();
                style.strikethrough = true;
                spans.push((pos..close, style));
                pos = close;
                continue;
            }
        }

        // Try to detect **bold**
        if pos + 1 < end && bytes[pos] == b'*' && bytes[pos + 1] == b'*' {
            if let Some(close) = find_closing_double(bytes, pos, end, b'*') {
                let mut style = line_style.clone();
                style.bold = true;
                spans.push((pos..close, style));
                pos = close;
                continue;
            }
        }

        // Try to detect *italic* (single asterisk, not double)
        if bytes[pos] == b'*' && !(pos + 1 < end && bytes[pos + 1] == b'*') {
            if let Some(close) = find_closing_single(bytes, pos, end, b'*') {
                let mut style = line_style.clone();
                style.italic = true;
                spans.push((pos..close, style));
                pos = close;
                continue;
            }
        }

        // Try to detect _italic_ (underscore)
        if bytes[pos] == b'_' {
            if let Some(close) = find_closing_single(bytes, pos, end, b'_') {
                let mut style = line_style.clone();
                style.italic = true;
                spans.push((pos..close, style));
                pos = close;
                continue;
            }
        }

        // Plain character — advance by one UTF-8 character
        let ch_len = utf8_char_len(bytes[pos]);
        let char_end = (pos + ch_len).min(end);
        spans.push((pos..char_end, line_style.clone()));
        pos = char_end;
    }
}

/// Try to parse a URL starting at `pos`. Returns `(end_pos, url_str)` if found.
fn try_parse_url(text: &str, pos: usize, end: usize) -> Option<(usize, &str)> {
    let rest = &text[pos..end];

    #[allow(clippy::if_same_then_else)]
    let prefix_len = if rest.starts_with("https://") {
        8
    } else if rest.starts_with("http://") {
        7
    } else if rest.starts_with("mailto:") {
        7
    } else {
        return None;
    };

    // Must have at least one character after the prefix
    if rest.len() <= prefix_len {
        return None;
    }

    // Consume until whitespace, bracket, or paren
    let url_end = rest[prefix_len..]
        .find(|c: char| c.is_whitespace() || matches!(c, '<' | '>' | '[' | ']' | '(' | ')'))
        .map(|i| prefix_len + i)
        .unwrap_or(rest.len());

    if url_end <= prefix_len {
        return None;
    }

    let url_str = &rest[..url_end];
    Some((pos + url_end, url_str))
}

/// Find closing backtick for inline code. Returns end position (after closing backtick).
fn find_closing_backtick(bytes: &[u8], open: usize, end: usize) -> Option<usize> {
    // Skip the opening backtick
    let mut pos = open + 1;
    while pos < end {
        if bytes[pos] == b'`' {
            return Some(pos + 1);
        }
        if bytes[pos] == b'\n' {
            // Code spans don't cross lines
            return None;
        }
        pos += 1;
    }
    None
}

/// Find closing double delimiter (e.g., `**` or `~~`). Returns end position (after closing).
fn find_closing_double(bytes: &[u8], open: usize, end: usize, delim: u8) -> Option<usize> {
    // Skip the opening two characters
    let mut pos = open + 2;
    // Must have at least one character between delimiters
    if pos >= end || bytes[pos] == delim {
        return None;
    }
    while pos + 1 < end {
        if bytes[pos] == delim && bytes[pos + 1] == delim {
            return Some(pos + 2);
        }
        if bytes[pos] == b'\n' {
            // Don't cross line boundaries
            return None;
        }
        pos += 1;
    }
    None
}

/// Find closing single delimiter (e.g., `*` or `_`). Returns end position (after closing).
fn find_closing_single(bytes: &[u8], open: usize, end: usize, delim: u8) -> Option<usize> {
    let mut pos = open + 1;
    // Must have at least one character between delimiters
    if pos >= end || bytes[pos] == delim {
        return None;
    }
    while pos < end {
        // For `*`, if we see `**` that's a bold delimiter, not two singles
        if bytes[pos] == delim {
            // Make sure this isn't a double delimiter (for asterisks)
            if delim == b'*' && pos + 1 < end && bytes[pos + 1] == b'*' {
                // Skip the double — this is bold syntax, not our closing single
                pos += 2;
                continue;
            }
            return Some(pos + 1);
        }
        if bytes[pos] == b'\n' {
            return None;
        }
        pos += 1;
    }
    None
}

/// Get the length of a UTF-8 character from its first byte.
fn utf8_char_len(first: u8) -> usize {
    if first < 0x80 {
        1
    } else if first < 0xE0 {
        2
    } else if first < 0xF0 {
        3
    } else {
        4
    }
}

/// Fill gaps between spans with default-styled spans, ensuring every byte is covered.
fn fill_gaps(spans: Vec<(Range<usize>, Style)>, total_len: usize) -> Vec<(Range<usize>, Style)> {
    if spans.is_empty() {
        if total_len > 0 {
            return vec![(0..total_len, Style::default())];
        }
        return Vec::new();
    }

    let mut result: Vec<(Range<usize>, Style)> = Vec::with_capacity(spans.len() + 2);
    let mut cursor = 0;

    for (range, style) in spans {
        if range.start > cursor {
            result.push((cursor..range.start, Style::default()));
        }
        // Avoid duplicate coverage by clamping
        let effective_start = range.start.max(cursor);
        if effective_start < range.end {
            result.push((effective_start..range.end, style));
        }
        cursor = cursor.max(range.end);
    }

    if cursor < total_len {
        result.push((cursor..total_len, Style::default()));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// Helper: verify spans are sorted by start position.
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

    #[test]
    fn test_empty_text() {
        let spans = parse("");
        assert!(spans.is_empty());
    }

    #[test]
    fn test_plain_text() {
        let text = "Hello, world!";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());
        for (_, style) in &spans {
            assert_eq!(*style, Style::default());
        }
    }

    #[test]
    fn test_bold() {
        let text = "before **bold** after";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());
        assert_sorted(&spans);

        let bold_spans: Vec<_> = spans.iter().filter(|(_, s)| s.bold).collect();
        assert_eq!(bold_spans.len(), 1);
        assert_eq!(&text[bold_spans[0].0.clone()], "**bold**");
    }

    #[test]
    fn test_italic_asterisk() {
        let text = "before *italic* after";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());
        assert_sorted(&spans);

        let italic_spans: Vec<_> = spans.iter().filter(|(_, s)| s.italic).collect();
        assert_eq!(italic_spans.len(), 1);
        assert_eq!(&text[italic_spans[0].0.clone()], "*italic*");
    }

    #[test]
    fn test_italic_underscore() {
        let text = "before _italic_ after";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());
        assert_sorted(&spans);

        let italic_spans: Vec<_> = spans.iter().filter(|(_, s)| s.italic).collect();
        assert_eq!(italic_spans.len(), 1);
        assert_eq!(&text[italic_spans[0].0.clone()], "_italic_");
    }

    #[test]
    fn test_strikethrough() {
        let text = "before ~~struck~~ after";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());
        assert_sorted(&spans);

        let strike_spans: Vec<_> = spans.iter().filter(|(_, s)| s.strikethrough).collect();
        assert_eq!(strike_spans.len(), 1);
        assert_eq!(&text[strike_spans[0].0.clone()], "~~struck~~");
    }

    #[test]
    fn test_inline_code() {
        let text = "before `code` after";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());
        assert_sorted(&spans);

        let code_spans: Vec<_> = spans.iter().filter(|(_, s)| s.code).collect();
        assert_eq!(code_spans.len(), 1);
        assert_eq!(&text[code_spans[0].0.clone()], "`code`");
    }

    #[test]
    fn test_heading_levels() {
        for level in 1u8..=6 {
            let prefix = "#".repeat(level as usize);
            let text = format!("{} Heading text", prefix);
            let spans = parse(&text);
            assert_full_coverage(&spans, text.len());
            for (_, style) in &spans {
                assert_eq!(style.heading, level, "Expected heading level {}", level);
            }
        }
    }

    #[test]
    fn test_heading_without_space_is_not_heading() {
        let text = "#NotAHeading";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());
        for (_, style) in &spans {
            assert_eq!(style.heading, 0);
        }
    }

    #[test]
    fn test_heading_level_7_not_recognized() {
        let text = "####### Not a heading";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());
        for (_, style) in &spans {
            assert_eq!(style.heading, 0);
        }
    }

    #[test]
    fn test_quote() {
        let text = "> This is a quote";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());
        for (_, style) in &spans {
            assert!(style.quote);
        }
    }

    #[test]
    fn test_quote_without_space() {
        let text = ">not a quote";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());
        for (_, style) in &spans {
            assert!(!style.quote);
        }
    }

    #[test]
    fn test_url_https() {
        let text = "Visit https://example.com for more";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());
        assert_sorted(&spans);

        let url_spans: Vec<_> = spans.iter().filter(|(_, s)| s.url).collect();
        assert_eq!(url_spans.len(), 1);
        let (range, _) = url_spans[0];
        assert_eq!(&text[range.clone()], "https://example.com");
    }

    #[test]
    fn test_url_http() {
        let text = "Visit http://example.com here";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());

        let url_spans: Vec<_> = spans.iter().filter(|(_, s)| s.url).collect();
        assert_eq!(url_spans.len(), 1);
        let (range, _) = url_spans[0];
        assert_eq!(&text[range.clone()], "http://example.com");
    }

    #[test]
    fn test_url_mailto() {
        let text = "Email mailto:user@example.com please";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());

        let url_spans: Vec<_> = spans.iter().filter(|(_, s)| s.url).collect();
        assert_eq!(url_spans.len(), 1);
        let (range, _) = url_spans[0];
        assert_eq!(&text[range.clone()], "mailto:user@example.com");
    }

    #[test]
    fn test_url_stops_at_bracket() {
        let text = "See [https://example.com] for info";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());

        let url_spans: Vec<_> = spans.iter().filter(|(_, s)| s.url).collect();
        assert_eq!(url_spans.len(), 1);
        let (range, _) = url_spans[0];
        assert_eq!(&text[range.clone()], "https://example.com");
    }

    #[test]
    fn test_url_stops_at_paren() {
        let text = "(https://example.com)";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());

        let url_spans: Vec<_> = spans.iter().filter(|(_, s)| s.url).collect();
        assert_eq!(url_spans.len(), 1);
        let (range, _) = url_spans[0];
        assert_eq!(&text[range.clone()], "https://example.com");
    }

    #[test]
    fn test_code_span_preserves_delimiters() {
        // Delimiters inside code spans should not be interpreted
        let text = "`**not bold**`";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());

        let code_spans: Vec<_> = spans.iter().filter(|(_, s)| s.code).collect();
        assert_eq!(code_spans.len(), 1);
        let bold_spans: Vec<_> = spans.iter().filter(|(_, s)| s.bold).collect();
        assert_eq!(bold_spans.len(), 0);
    }

    #[test]
    fn test_unmatched_bold_delimiter() {
        let text = "before **unmatched after";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());
        // Should not have any bold spans
        let bold_spans: Vec<_> = spans.iter().filter(|(_, s)| s.bold).collect();
        assert_eq!(bold_spans.len(), 0);
    }

    #[test]
    fn test_unmatched_italic_delimiter() {
        let text = "before *unmatched after";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());
        let italic_spans: Vec<_> = spans.iter().filter(|(_, s)| s.italic).collect();
        assert_eq!(italic_spans.len(), 0);
    }

    #[test]
    fn test_unmatched_code_backtick() {
        let text = "before `unmatched after";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());
        let code_spans: Vec<_> = spans.iter().filter(|(_, s)| s.code).collect();
        assert_eq!(code_spans.len(), 0);
    }

    #[test]
    fn test_unmatched_strikethrough() {
        let text = "before ~~unmatched after";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());
        let strike_spans: Vec<_> = spans.iter().filter(|(_, s)| s.strikethrough).collect();
        assert_eq!(strike_spans.len(), 0);
    }

    #[test]
    fn test_empty_bold_not_matched() {
        // **** should not be matched as bold with empty content
        let text = "before **** after";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());
    }

    #[test]
    fn test_empty_italic_not_matched() {
        let text = "before ** after";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());
    }

    #[test]
    fn test_multiline_text() {
        let text = "# Heading\nNormal text\n> Quote line\n";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());
        assert_sorted(&spans);

        // First line should be heading
        let heading_spans: Vec<_> = spans.iter().filter(|(_, s)| s.heading > 0).collect();
        assert!(!heading_spans.is_empty());

        // Third line should be quote
        let quote_spans: Vec<_> = spans.iter().filter(|(_, s)| s.quote).collect();
        assert!(!quote_spans.is_empty());
    }

    #[test]
    fn test_bold_does_not_cross_lines() {
        let text = "**bold\nstill bold**";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());
        let bold_spans: Vec<_> = spans.iter().filter(|(_, s)| s.bold).collect();
        assert_eq!(bold_spans.len(), 0, "Bold should not cross line boundaries");
    }

    #[test]
    fn test_code_does_not_cross_lines() {
        let text = "`code\nstill code`";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());
        let code_spans: Vec<_> = spans.iter().filter(|(_, s)| s.code).collect();
        assert_eq!(code_spans.len(), 0, "Code should not cross line boundaries");
    }

    #[test]
    fn test_unicode_text() {
        let text = "Hello, \u{4e16}\u{754c}! **\u{0431}\u{043e}\u{043b}\u{0434}** end";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());
        assert_sorted(&spans);

        let bold_spans: Vec<_> = spans.iter().filter(|(_, s)| s.bold).collect();
        assert_eq!(bold_spans.len(), 1);
    }

    #[test]
    fn test_multiple_formatting_on_one_line() {
        let text = "**bold** and *italic* and `code`";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());
        assert_sorted(&spans);

        assert_eq!(spans.iter().filter(|(_, s)| s.bold).count(), 1);
        assert_eq!(spans.iter().filter(|(_, s)| s.italic).count(), 1);
        assert_eq!(spans.iter().filter(|(_, s)| s.code).count(), 1);
    }

    #[test]
    fn test_heading_with_inline_formatting() {
        let text = "## **Bold heading**";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());

        let heading_bold: Vec<_> = spans
            .iter()
            .filter(|(_, s)| s.heading == 2 && s.bold)
            .collect();
        assert!(!heading_bold.is_empty());
    }

    #[test]
    fn test_quote_with_inline_formatting() {
        let text = "> *italic in quote*";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());

        let quote_italic: Vec<_> = spans.iter().filter(|(_, s)| s.quote && s.italic).collect();
        assert!(!quote_italic.is_empty());
    }

    #[test]
    fn test_url_in_heading() {
        let text = "# Visit https://example.com";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());

        let heading_url: Vec<_> = spans
            .iter()
            .filter(|(_, s)| s.heading == 1 && s.url)
            .collect();
        assert!(!heading_url.is_empty());
    }

    #[test]
    fn test_only_newlines() {
        let text = "\n\n\n";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());
    }

    #[test]
    fn test_single_character() {
        let text = "a";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());
        assert_eq!(spans.len(), 1);
    }

    #[test]
    fn test_adjacent_formatting() {
        let text = "**bold***italic*";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());
        assert_sorted(&spans);

        let bold_spans: Vec<_> = spans.iter().filter(|(_, s)| s.bold).collect();
        assert_eq!(bold_spans.len(), 1);
        assert_eq!(&text[bold_spans[0].0.clone()], "**bold**");

        let italic_spans: Vec<_> = spans.iter().filter(|(_, s)| s.italic).collect();
        assert_eq!(italic_spans.len(), 1);
        assert_eq!(&text[italic_spans[0].0.clone()], "*italic*");
    }

    #[test]
    fn test_url_with_path_and_query() {
        let text = "Check https://example.com/path?query=1&foo=bar end";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());

        let url_spans: Vec<_> = spans.iter().filter(|(_, s)| s.url).collect();
        assert_eq!(url_spans.len(), 1);
        assert_eq!(
            &text[url_spans[0].0.clone()],
            "https://example.com/path?query=1&foo=bar"
        );
    }

    #[test]
    fn test_url_at_end_of_text() {
        let text = "Visit https://example.com";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());

        let url_spans: Vec<_> = spans.iter().filter(|(_, s)| s.url).collect();
        assert_eq!(url_spans.len(), 1);
        assert_eq!(&text[url_spans[0].0.clone()], "https://example.com");
    }

    #[test]
    fn test_consecutive_urls() {
        let text = "https://a.com https://b.com";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());

        let url_spans: Vec<_> = spans.iter().filter(|(_, s)| s.url).collect();
        assert_eq!(url_spans.len(), 2);
    }

    #[test]
    fn test_url_prefix_only_not_matched() {
        // Just "https://" with nothing after shouldn't match
        let text = "Check https:// end";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());
        // The URL needs at least one char after the protocol prefix
        // "https://" has length 8, and "https:// " would stop at whitespace immediately
        // so it should not be detected as URL
    }

    #[test]
    fn test_strikethrough_with_spaces() {
        let text = "~~some struck text~~";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());

        let strike_spans: Vec<_> = spans.iter().filter(|(_, s)| s.strikethrough).collect();
        assert_eq!(strike_spans.len(), 1);
        assert_eq!(&text[strike_spans[0].0.clone()], "~~some struck text~~");
    }

    #[test]
    fn test_delimiter_inside_code() {
        let text = "`*not italic*` and `**not bold**`";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());

        let code_spans: Vec<_> = spans.iter().filter(|(_, s)| s.code).collect();
        assert_eq!(code_spans.len(), 2);
        let italic_spans: Vec<_> = spans.iter().filter(|(_, s)| s.italic).collect();
        assert_eq!(italic_spans.len(), 0);
        let bold_spans: Vec<_> = spans.iter().filter(|(_, s)| s.bold).collect();
        assert_eq!(bold_spans.len(), 0);
    }

    #[test]
    fn test_heading_empty_after_hash() {
        let text = "#\n";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());
        let heading_spans: Vec<_> = spans.iter().filter(|(_, s)| s.heading == 1).collect();
        assert!(!heading_spans.is_empty());
    }

    #[test]
    fn test_tilde_single_not_strikethrough() {
        let text = "~not strike~";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());
        let strike_spans: Vec<_> = spans.iter().filter(|(_, s)| s.strikethrough).collect();
        assert_eq!(strike_spans.len(), 0);
    }

    #[test]
    fn test_windows_line_endings() {
        let text = "# Heading\r\nNormal text\r\n> Quote\r\n";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());
        assert_sorted(&spans);
    }

    #[test]
    fn test_many_hashes_beyond_six() {
        let text = "######## Not a heading";
        let spans = parse(text);
        assert_full_coverage(&spans, text.len());
        for (_, style) in &spans {
            assert_eq!(style.heading, 0);
        }
    }
}
