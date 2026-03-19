//! Text manipulation helpers for markdown formatting shortcuts.
//!
//! All functions work with **char offsets** for cursor positions (matching
//! egui's `CCursor.index`) and return content as `String`.

use std::ops::Range;

/// Toggle a delimiter around the selected text.
///
/// If the selection is already wrapped with `delimiter`, remove it (unwrap).
/// Otherwise, wrap the selection with `delimiter` on both sides.
///
/// # Arguments
/// - `content`: the full text content
/// - `start`: selection start (char offset)
/// - `end`: selection end (char offset, may equal start for cursor-only)
/// - `delimiter`: the markdown delimiter (e.g., `**`, `*`, `~~`, `` ` ``)
///
/// # Returns
/// `(new_content, new_start, new_end)` with adjusted char positions.
pub fn toggle_surrounding(
    content: &str,
    start: usize,
    end: usize,
    delimiter: &str,
) -> (String, usize, usize) {
    let chars: Vec<char> = content.chars().collect();
    let total = chars.len();
    let dlen = delimiter.chars().count();

    let start = start.min(total);
    let end = end.min(total);
    let (start, end) = if start <= end {
        (start, end)
    } else {
        (end, start)
    };

    // Check if the selection is already wrapped with the delimiter
    if start >= dlen && end + dlen <= total {
        let prefix: String = chars[start - dlen..start].iter().collect();
        let suffix: String = chars[end..end + dlen].iter().collect();
        if prefix == delimiter && suffix == delimiter {
            // Unwrap: remove the delimiter on both sides
            let before: String = chars[..start - dlen].iter().collect();
            let middle: String = chars[start..end].iter().collect();
            let after: String = chars[end + dlen..].iter().collect();
            let new_content = format!("{}{}{}", before, middle, after);
            return (new_content, start - dlen, end - dlen);
        }
    }

    // Wrap: insert delimiter around selection
    let before: String = chars[..start].iter().collect();
    let middle: String = chars[start..end].iter().collect();
    let after: String = chars[end..].iter().collect();
    let new_content = format!("{}{}{}{}{}", before, delimiter, middle, delimiter, after);

    (new_content, start + dlen, end + dlen)
}

/// Add 4 spaces at the start of each line that intersects the selection.
///
/// # Arguments
/// - `content`: the full text content
/// - `start`: selection start (char offset)
/// - `end`: selection end (char offset)
///
/// # Returns
/// `(new_content, new_start, new_end)` with adjusted char positions.
pub fn indent_lines(content: &str, start: usize, end: usize) -> (String, usize, usize) {
    let chars: Vec<char> = content.chars().collect();
    let total = chars.len();
    let start = start.min(total);
    let end = end.min(total);
    let (start, end) = if start <= end {
        (start, end)
    } else {
        (end, start)
    };

    let lines = split_lines_with_offsets(&chars);
    let indent = "    "; // 4 spaces
    let indent_len = 4usize;

    let mut result = String::new();
    let mut new_start = start;
    let mut new_end = end;
    let mut cumulative_shift = 0usize;

    for (line_start, line_end, line_chars) in &lines {
        let line_intersects =
            (*line_start < end && *line_end > start) || (*line_start == start && *line_end == end);
        let cursor_on_line = start == end && *line_start <= start && start <= *line_end;

        if line_intersects || cursor_on_line {
            result.push_str(indent);
            let line_text: String = line_chars.iter().collect();
            result.push_str(&line_text);

            // Adjust start if it's on or after this line
            if *line_start <= start {
                new_start = start + cumulative_shift + indent_len;
            } else if cumulative_shift > 0 {
                // start is before this line, but we've already shifted
                // new_start was set in a previous iteration
            }

            // Adjust end similarly
            if *line_start <= end {
                new_end = end + cumulative_shift + indent_len;
            }

            cumulative_shift += indent_len;
        } else {
            let line_text: String = line_chars.iter().collect();
            result.push_str(&line_text);
        }
    }

    (result, new_start, new_end)
}

/// Remove up to 4 leading spaces from each line that intersects the selection.
///
/// # Arguments
/// - `content`: the full text content
/// - `start`: selection start (char offset)
/// - `end`: selection end (char offset)
///
/// # Returns
/// `(new_content, new_start, new_end)` with adjusted char positions.
pub fn outdent_lines(content: &str, start: usize, end: usize) -> (String, usize, usize) {
    let chars: Vec<char> = content.chars().collect();
    let total = chars.len();
    let start = start.min(total);
    let end = end.min(total);
    let (start, end) = if start <= end {
        (start, end)
    } else {
        (end, start)
    };

    let lines = split_lines_with_offsets(&chars);

    let mut result = String::new();
    let mut new_start = start;
    let mut new_end = end;
    let mut cumulative_removed = 0usize;

    for (line_start, line_end, line_chars) in &lines {
        let line_intersects =
            (*line_start < end && *line_end > start) || (*line_start == start && *line_end == end);
        let cursor_on_line = start == end && *line_start <= start && start <= *line_end;

        if line_intersects || cursor_on_line {
            // Count leading spaces (up to 4)
            let spaces = line_chars.iter().take(4).take_while(|&&c| c == ' ').count();
            let line_text: String = line_chars[spaces..].iter().collect();
            result.push_str(&line_text);

            // Adjust start: if start is on this line, it moves back by
            // min(spaces, start - line_start) to avoid going before line start
            if *line_start <= start && start < *line_end {
                let offset_in_line = start - line_start;
                let actual_shift = spaces.min(offset_in_line);
                new_start = start - cumulative_removed - actual_shift;
            } else if *line_start > start {
                // start is before this line, already set
            }

            // Adjust end similarly
            if *line_start <= end && end <= *line_end {
                let offset_in_line = end - line_start;
                let actual_shift = spaces.min(offset_in_line);
                new_end = end - cumulative_removed - actual_shift;
            } else if *line_end <= end {
                // end is after this line, will be adjusted in a later iteration
                // or end is exactly at line boundary
                new_end = end - cumulative_removed - spaces;
            }

            cumulative_removed += spaces;
        } else {
            let line_text: String = line_chars.iter().collect();
            result.push_str(&line_text);
        }
    }

    (result, new_start, new_end)
}

/// Auto-indent: if a newline was just inserted, detect previous line's
/// indentation and list prefix, then insert matching whitespace.
///
/// # Arguments
/// - `content`: the full text content (after the newline was inserted)
/// - `cursor`: current cursor position (char offset, after the newline)
///
/// # Returns
/// `Some((new_content, new_cursor))` if indentation was inserted, `None` otherwise.
pub fn auto_indent(content: &str, cursor: usize) -> Option<(String, usize)> {
    let chars: Vec<char> = content.chars().collect();

    // The cursor should be right after a newline
    if cursor == 0 || cursor > chars.len() {
        return None;
    }
    if chars[cursor - 1] != '\n' {
        return None;
    }

    // Find the previous line
    let prev_line_end = cursor - 1; // position of the newline
    let prev_line_start = if prev_line_end == 0 {
        0
    } else {
        chars[..prev_line_end]
            .iter()
            .rposition(|&c| c == '\n')
            .map(|p| p + 1)
            .unwrap_or(0)
    };

    let prev_line: String = chars[prev_line_start..prev_line_end].iter().collect();

    // Detect indentation
    let indent: String = prev_line
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect();

    // Detect list prefix after indentation
    let after_indent = &prev_line[indent.len()..];
    let list_prefix = detect_list_prefix(after_indent);

    let insertion = format!("{}{}", indent, list_prefix);
    if insertion.is_empty() {
        return None;
    }

    // Insert the indentation at cursor position
    let before: String = chars[..cursor].iter().collect();
    let after: String = chars[cursor..].iter().collect();
    let new_content = format!("{}{}{}", before, insertion, after);
    let new_cursor = cursor + insertion.chars().count();

    Some((new_content, new_cursor))
}

/// Find all URLs in text, returning byte-offset ranges.
pub fn find_urls(text: &str) -> Vec<Range<usize>> {
    let mut urls = Vec::new();
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut pos = 0;

    while pos < len {
        let rest = &text[pos..];
        #[allow(clippy::if_same_then_else)]
        let prefix_len = if rest.starts_with("https://") {
            8
        } else if rest.starts_with("http://") {
            7
        } else if rest.starts_with("mailto:") {
            7
        } else {
            pos += utf8_char_len(bytes[pos]);
            continue;
        };

        if rest.len() <= prefix_len {
            pos += prefix_len;
            continue;
        }

        // Check the first char after prefix isn't a URL-terminator
        let after_prefix = &rest[prefix_len..];
        let first_after = after_prefix.chars().next();
        if first_after.is_none() || first_after.unwrap().is_whitespace() {
            pos += prefix_len;
            continue;
        }

        // Consume until whitespace/bracket/paren
        let url_end = after_prefix
            .find(|c: char| c.is_whitespace() || matches!(c, '<' | '>' | '[' | ']' | '(' | ')'))
            .map(|i| prefix_len + i)
            .unwrap_or(rest.len());

        if url_end > prefix_len {
            urls.push(pos..pos + url_end);
            pos += url_end;
        } else {
            pos += prefix_len;
        }
    }

    urls
}

// ─── Internal helpers ───────────────────────────────────────────────────────

/// Split chars into lines, preserving newline characters at the end of each line.
/// Returns Vec<(line_start_char, line_end_char, chars_including_newline)>.
fn split_lines_with_offsets(chars: &[char]) -> Vec<(usize, usize, Vec<char>)> {
    let mut lines = Vec::new();
    let mut start = 0;

    for (i, &c) in chars.iter().enumerate() {
        if c == '\n' {
            lines.push((start, i + 1, chars[start..=i].to_vec()));
            start = i + 1;
        }
    }

    // Last line (may not end with newline)
    if start <= chars.len() {
        lines.push((start, chars.len(), chars[start..].to_vec()));
    }

    lines
}

/// Detect a list prefix at the start of a line (after indentation).
/// Returns the prefix to repeat (e.g., "- ", "* ", "1. ", "2. ", etc.)
fn detect_list_prefix(line: &str) -> String {
    let trimmed = line.trim_start();

    // Unordered list: - or *
    if trimmed.starts_with("- ") {
        return "- ".to_string();
    }
    if trimmed.starts_with("* ") {
        return "* ".to_string();
    }

    // Ordered list: digits followed by . and space
    let digits: String = trimmed.chars().take_while(|c| c.is_ascii_digit()).collect();
    if !digits.is_empty() {
        let rest = &trimmed[digits.len()..];
        if rest.starts_with(". ") {
            // Increment the number
            if let Ok(num) = digits.parse::<u64>() {
                return format!("{}. ", num + 1);
            }
        }
    }

    String::new()
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

#[cfg(test)]
mod tests {
    use super::*;

    // ─── toggle_surrounding tests ───────────────────────────────────────

    #[test]
    fn test_toggle_wrap_selection() {
        let (result, start, end) = toggle_surrounding("hello world", 0, 5, "**");
        assert_eq!(result, "**hello** world");
        assert_eq!(start, 2);
        assert_eq!(end, 7);
    }

    #[test]
    fn test_toggle_unwrap_selection() {
        let (result, start, end) = toggle_surrounding("**hello** world", 2, 7, "**");
        assert_eq!(result, "hello world");
        assert_eq!(start, 0);
        assert_eq!(end, 5);
    }

    #[test]
    fn test_toggle_no_selection_inserts_delimiters() {
        let (result, start, end) = toggle_surrounding("hello world", 5, 5, "**");
        assert_eq!(result, "hello**** world");
        assert_eq!(start, 7);
        assert_eq!(end, 7);
    }

    #[test]
    fn test_toggle_single_char_delimiter() {
        let (result, start, end) = toggle_surrounding("hello world", 0, 5, "*");
        assert_eq!(result, "*hello* world");
        assert_eq!(start, 1);
        assert_eq!(end, 6);
    }

    #[test]
    fn test_toggle_unwrap_single_char_delimiter() {
        let (result, start, end) = toggle_surrounding("*hello* world", 1, 6, "*");
        assert_eq!(result, "hello world");
        assert_eq!(start, 0);
        assert_eq!(end, 5);
    }

    #[test]
    fn test_toggle_backtick() {
        let (result, start, end) = toggle_surrounding("some code here", 5, 9, "`");
        assert_eq!(result, "some `code` here");
        assert_eq!(start, 6);
        assert_eq!(end, 10);
    }

    #[test]
    fn test_toggle_strikethrough() {
        let (result, start, end) = toggle_surrounding("remove this text", 7, 11, "~~");
        assert_eq!(result, "remove ~~this~~ text");
        assert_eq!(start, 9);
        assert_eq!(end, 13);
    }

    #[test]
    fn test_toggle_at_start() {
        let (result, _, _) = toggle_surrounding("hello", 0, 5, "**");
        assert_eq!(result, "**hello**");
    }

    #[test]
    fn test_toggle_at_end() {
        let (result, _, _) = toggle_surrounding("hello", 5, 5, "**");
        assert_eq!(result, "hello****");
    }

    #[test]
    fn test_toggle_empty_string() {
        let (result, start, end) = toggle_surrounding("", 0, 0, "**");
        assert_eq!(result, "****");
        assert_eq!(start, 2);
        assert_eq!(end, 2);
    }

    #[test]
    fn test_toggle_unicode() {
        let text = "\u{4e16}\u{754c}hello";
        let (result, start, end) = toggle_surrounding(text, 0, 2, "**");
        assert_eq!(result, "**\u{4e16}\u{754c}**hello");
        assert_eq!(start, 2);
        assert_eq!(end, 4);
    }

    // ─── indent_lines tests ────────────────────────────────────────────

    #[test]
    fn test_indent_single_line() {
        let (result, _, _) = indent_lines("hello", 0, 5);
        assert_eq!(result, "    hello");
    }

    #[test]
    fn test_indent_multiple_lines() {
        let (result, _, _) = indent_lines("line1\nline2\nline3", 0, 17);
        assert_eq!(result, "    line1\n    line2\n    line3");
    }

    #[test]
    fn test_indent_partial_selection() {
        // Select only "line2"
        let (result, _, _) = indent_lines("line1\nline2\nline3", 6, 11);
        assert_eq!(result, "line1\n    line2\nline3");
    }

    // ─── outdent_lines tests ───────────────────────────────────────────

    #[test]
    fn test_outdent_single_line() {
        let (result, _, _) = outdent_lines("    hello", 0, 9);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_outdent_partial_spaces() {
        let (result, _, _) = outdent_lines("  hello", 0, 7);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_outdent_no_spaces() {
        let (result, _, _) = outdent_lines("hello", 0, 5);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_outdent_more_than_four_spaces() {
        let (result, _, _) = outdent_lines("      hello", 0, 11);
        assert_eq!(result, "  hello");
    }

    #[test]
    fn test_outdent_multiple_lines() {
        let (result, _, _) = outdent_lines("    line1\n    line2", 0, 19);
        assert_eq!(result, "line1\nline2");
    }

    // ─── auto_indent tests ─────────────────────────────────────────────

    #[test]
    fn test_auto_indent_basic() {
        let content = "    hello\n";
        let cursor = content.len(); // right after the newline
        let result = auto_indent(content, cursor);
        assert!(result.is_some());
        let (new_content, new_cursor) = result.unwrap();
        assert_eq!(new_content, "    hello\n    ");
        assert_eq!(new_cursor, 14);
    }

    #[test]
    fn test_auto_indent_list_dash() {
        let content = "  - item\n";
        let cursor = content.len();
        let result = auto_indent(content, cursor);
        assert!(result.is_some());
        let (new_content, _) = result.unwrap();
        assert_eq!(new_content, "  - item\n  - ");
    }

    #[test]
    fn test_auto_indent_list_asterisk() {
        let content = "* item\n";
        let cursor = content.len();
        let result = auto_indent(content, cursor);
        assert!(result.is_some());
        let (new_content, _) = result.unwrap();
        assert_eq!(new_content, "* item\n* ");
    }

    #[test]
    fn test_auto_indent_ordered_list() {
        let content = "1. first\n";
        let cursor = content.len();
        let result = auto_indent(content, cursor);
        assert!(result.is_some());
        let (new_content, _) = result.unwrap();
        assert_eq!(new_content, "1. first\n2. ");
    }

    #[test]
    fn test_auto_indent_ordered_list_increment() {
        let content = "5. fifth\n";
        let cursor = content.len();
        let result = auto_indent(content, cursor);
        assert!(result.is_some());
        let (new_content, _) = result.unwrap();
        assert_eq!(new_content, "5. fifth\n6. ");
    }

    #[test]
    fn test_auto_indent_no_indent() {
        let content = "hello\n";
        let cursor = content.len();
        let result = auto_indent(content, cursor);
        assert!(result.is_none());
    }

    #[test]
    fn test_auto_indent_cursor_not_after_newline() {
        let content = "hello\nworld";
        let cursor = 3; // middle of "hello"
        let result = auto_indent(content, cursor);
        assert!(result.is_none());
    }

    #[test]
    fn test_auto_indent_empty_string() {
        let result = auto_indent("", 0);
        assert!(result.is_none());
    }

    #[test]
    fn test_auto_indent_tab_indent() {
        let content = "\thello\n";
        let cursor = content.len();
        let result = auto_indent(content, cursor);
        assert!(result.is_some());
        let (new_content, _) = result.unwrap();
        assert_eq!(new_content, "\thello\n\t");
    }

    // ─── find_urls tests ───────────────────────────────────────────────

    #[test]
    fn test_find_urls_https() {
        let urls = find_urls("Visit https://example.com today");
        assert_eq!(urls.len(), 1);
        assert_eq!(
            &"Visit https://example.com today"[urls[0].clone()],
            "https://example.com"
        );
    }

    #[test]
    fn test_find_urls_http() {
        let urls = find_urls("Check http://test.org here");
        assert_eq!(urls.len(), 1);
        assert_eq!(
            &"Check http://test.org here"[urls[0].clone()],
            "http://test.org"
        );
    }

    #[test]
    fn test_find_urls_mailto() {
        let urls = find_urls("Email mailto:user@example.com");
        assert_eq!(urls.len(), 1);
        assert_eq!(
            &"Email mailto:user@example.com"[urls[0].clone()],
            "mailto:user@example.com"
        );
    }

    #[test]
    fn test_find_urls_multiple() {
        let text = "https://a.com and http://b.org end";
        let urls = find_urls(text);
        assert_eq!(urls.len(), 2);
        assert_eq!(&text[urls[0].clone()], "https://a.com");
        assert_eq!(&text[urls[1].clone()], "http://b.org");
    }

    #[test]
    fn test_find_urls_none() {
        let urls = find_urls("No URLs here");
        assert!(urls.is_empty());
    }

    #[test]
    fn test_find_urls_with_path() {
        let text = "https://example.com/path/to/page?q=1&r=2";
        let urls = find_urls(text);
        assert_eq!(urls.len(), 1);
        assert_eq!(&text[urls[0].clone()], text);
    }

    #[test]
    fn test_find_urls_stops_at_bracket() {
        let text = "[https://example.com]";
        let urls = find_urls(text);
        assert_eq!(urls.len(), 1);
        assert_eq!(&text[urls[0].clone()], "https://example.com");
    }

    #[test]
    fn test_find_urls_stops_at_paren() {
        let text = "(https://example.com)";
        let urls = find_urls(text);
        assert_eq!(urls.len(), 1);
        assert_eq!(&text[urls[0].clone()], "https://example.com");
    }

    #[test]
    fn test_find_urls_at_end() {
        let text = "Visit https://example.com";
        let urls = find_urls(text);
        assert_eq!(urls.len(), 1);
        assert_eq!(&text[urls[0].clone()], "https://example.com");
    }

    #[test]
    fn test_find_urls_empty_string() {
        let urls = find_urls("");
        assert!(urls.is_empty());
    }

    // ─── detect_list_prefix tests ──────────────────────────────────────

    #[test]
    fn test_detect_list_dash() {
        assert_eq!(detect_list_prefix("- item"), "- ");
    }

    #[test]
    fn test_detect_list_asterisk() {
        assert_eq!(detect_list_prefix("* item"), "* ");
    }

    #[test]
    fn test_detect_list_ordered() {
        assert_eq!(detect_list_prefix("1. item"), "2. ");
    }

    #[test]
    fn test_detect_list_ordered_large() {
        assert_eq!(detect_list_prefix("99. item"), "100. ");
    }

    #[test]
    fn test_detect_no_list() {
        assert_eq!(detect_list_prefix("plain text"), "");
    }

    #[test]
    fn test_detect_list_no_space_after_dash() {
        assert_eq!(detect_list_prefix("-no space"), "");
    }
}
