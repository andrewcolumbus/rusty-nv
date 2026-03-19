use crate::markup;
use egui::text::LayoutJob;
use egui::{Color32, FontFamily, FontId, Stroke, TextFormat};
use std::ops::Range;

/// The kind of a text span for highlighting purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanKind {
    Normal,
    SearchMatch,
    ActiveSearchMatch,
    WikiLink,
}

/// A span of text with a classification for highlighting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightSpan {
    /// Byte range into the source text.
    pub byte_range: Range<usize>,
    /// What kind of highlighting to apply.
    pub kind: SpanKind,
}

/// Colors used for highlighting different span kinds.
#[derive(Debug, Clone, Copy)]
pub struct HighlightColors {
    pub default_text: Color32,
    pub search_bg: Color32,
    pub active_search_bg: Color32,
    pub link_color: Color32,
}

/// Find all case-insensitive occurrences of `query` in `text`.
/// Returns byte-offset ranges. Returns empty vec if query is empty.
pub fn find_search_matches(text: &str, query: &str) -> Vec<Range<usize>> {
    if query.is_empty() {
        return Vec::new();
    }

    let text_lower = text.to_lowercase();
    let query_lower = query.to_lowercase();
    let mut matches = Vec::new();
    let mut search_start = 0;

    while let Some(pos) = text_lower[search_start..].find(&query_lower) {
        let byte_start = search_start + pos;
        // We need to find the byte end in the *original* text that corresponds
        // to the same number of characters as the query match in the lowercased text.
        // Since to_lowercase can change byte lengths for some chars, we map back
        // through the original text using character counts.
        let match_char_count = query_lower.chars().count();
        let orig_char_start = text_lower[..byte_start].chars().count();

        // Find byte_start in original text
        let orig_byte_start = char_offset_to_byte_offset(text, orig_char_start);
        // Find byte_end in original text
        let orig_byte_end = char_offset_to_byte_offset(text, orig_char_start + match_char_count);

        matches.push(orig_byte_start..orig_byte_end);
        // Advance past this match (by at least one character to avoid infinite loops)
        let advance = text_lower[byte_start..]
            .chars()
            .next()
            .map(|c| c.len_utf8())
            .unwrap_or(1);
        search_start = byte_start + advance;
    }

    matches
}

/// Find all `[[...]]` wiki link byte ranges using a state machine.
/// Returns the full range including the brackets.
pub fn find_wiki_links(text: &str) -> Vec<Range<usize>> {
    let mut links = Vec::new();
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // Look for opening [[
        if i + 1 < len && bytes[i] == b'[' && bytes[i + 1] == b'[' {
            let start = i;
            i += 2; // skip past [[
                    // Scan for closing ]]
            let mut found_close = false;
            while i + 1 < len {
                if bytes[i] == b']' && bytes[i + 1] == b']' {
                    // Found closing ]]
                    let end = i + 2;
                    // Only include non-empty links (i.e., not [[]])
                    if end - start > 4 {
                        links.push(start..end);
                    }
                    i = end;
                    found_close = true;
                    break;
                }
                if bytes[i] == b'\n' {
                    // Don't allow wiki links to span lines
                    break;
                }
                i += 1;
            }
            if !found_close {
                // No closing found, move past the opening [[
                i = start + 2;
            }
        } else {
            i += 1;
        }
    }

    links
}

/// Build a merged, non-overlapping list of `HighlightSpan`s covering the entire text.
///
/// Wiki links take precedence over search matches. `active_match_index` (0-based)
/// specifies which search match (in order) should be marked as `ActiveSearchMatch`.
pub fn find_spans(
    text: &str,
    query: &str,
    active_match_index: Option<usize>,
) -> Vec<HighlightSpan> {
    let text_len = text.len();
    if text_len == 0 {
        return Vec::new();
    }

    let search_matches = find_search_matches(text, query);
    let wiki_links = find_wiki_links(text);

    // Collect all "special" ranges with their kind
    struct TaggedRange {
        range: Range<usize>,
        kind: SpanKind,
    }

    let mut tagged: Vec<TaggedRange> = Vec::new();

    // Wiki links first (higher priority)
    for r in &wiki_links {
        tagged.push(TaggedRange {
            range: r.clone(),
            kind: SpanKind::WikiLink,
        });
    }

    // Search matches, but skip any that overlap with wiki links
    for (i, r) in search_matches.iter().enumerate() {
        let overlaps_wiki = wiki_links
            .iter()
            .any(|w| r.start < w.end && r.end > w.start);
        if !overlaps_wiki {
            let kind = if active_match_index == Some(i) {
                SpanKind::ActiveSearchMatch
            } else {
                SpanKind::SearchMatch
            };
            tagged.push(TaggedRange {
                range: r.clone(),
                kind,
            });
        }
    }

    // Sort by start position
    tagged.sort_by_key(|t| t.range.start);

    // Build spans covering the entire text
    let mut spans = Vec::new();
    let mut pos = 0;

    for t in &tagged {
        if t.range.start > pos {
            // Normal gap before this special range
            spans.push(HighlightSpan {
                byte_range: pos..t.range.start,
                kind: SpanKind::Normal,
            });
        }
        // Ensure we don't go backwards (in case of overlapping ranges, though
        // we've tried to prevent this)
        if t.range.start >= pos {
            spans.push(HighlightSpan {
                byte_range: t.range.clone(),
                kind: t.kind,
            });
            pos = t.range.end;
        }
    }

    // Trailing normal text
    if pos < text_len {
        spans.push(HighlightSpan {
            byte_range: pos..text_len,
            kind: SpanKind::Normal,
        });
    }

    spans
}

/// Build a `LayoutJob` from spans. Every byte of the text must be covered by exactly
/// one `LayoutSection`. `LayoutSection.byte_range` uses **byte** offsets.
#[allow(dead_code)]
pub fn build_layout_job(
    text: &str,
    spans: &[HighlightSpan],
    font_id: FontId,
    colors: &HighlightColors,
    wrap_width: f32,
) -> LayoutJob {
    let mut job = LayoutJob {
        text: text.to_string(),
        break_on_newline: true,
        wrap: egui::text::TextWrapping {
            max_width: wrap_width,
            ..Default::default()
        },
        ..Default::default()
    };

    if spans.is_empty() {
        // If no spans, cover the entire text as normal
        if !text.is_empty() {
            job.sections.push(egui::text::LayoutSection {
                leading_space: 0.0,
                byte_range: 0..text.len(),
                format: TextFormat {
                    font_id: font_id.clone(),
                    color: colors.default_text,
                    ..Default::default()
                },
            });
        }
        return job;
    }

    for span in spans {
        let format = match span.kind {
            SpanKind::Normal => TextFormat {
                font_id: font_id.clone(),
                color: colors.default_text,
                ..Default::default()
            },
            SpanKind::SearchMatch => TextFormat {
                font_id: font_id.clone(),
                color: colors.default_text,
                background: colors.search_bg,
                ..Default::default()
            },
            SpanKind::ActiveSearchMatch => TextFormat {
                font_id: font_id.clone(),
                color: colors.default_text,
                background: colors.active_search_bg,
                ..Default::default()
            },
            SpanKind::WikiLink => TextFormat {
                font_id: font_id.clone(),
                color: colors.link_color,
                underline: egui::Stroke::new(1.0, colors.link_color),
                ..Default::default()
            },
        };

        job.sections.push(egui::text::LayoutSection {
            leading_space: 0.0,
            byte_range: span.byte_range.clone(),
            format,
        });
    }

    job
}

/// Build a `LayoutJob` that combines markdown formatting with search/wiki highlighting.
///
/// Markdown formatting sets font, italic, strikethrough, code background, etc.
/// Search highlighting overlays a background color. Wiki links override the text color.
pub fn build_combined_layout_job(
    text: &str,
    search_query: &str,
    active_match: Option<usize>,
    base_font_size: f32,
    ui_style: &egui::Style,
    colors: &HighlightColors,
    wrap_width: f32,
) -> LayoutJob {
    if text.is_empty() {
        return LayoutJob::default();
    }

    let markup_spans = markup::parse(text);
    let highlight_spans = find_spans(text, search_query, active_match);

    let text_color = ui_style
        .visuals
        .override_text_color
        .unwrap_or(colors.default_text);
    let code_bg = ui_style.visuals.code_bg_color;
    let link_color = colors.link_color;
    let dimmed_color = if ui_style.visuals.dark_mode {
        Color32::from_rgba_premultiplied(160, 160, 175, 255)
    } else {
        Color32::from_rgba_premultiplied(100, 100, 110, 255)
    };

    let default_font = FontId::new(base_font_size, FontFamily::Monospace);

    // Merge the two sorted, non-overlapping span lists.
    // Both cover all bytes of the text.
    let mut sections = Vec::new();
    let mut mi = 0usize; // markup index
    let mut hi = 0usize; // highlight index
    let mut pos = 0usize;
    let text_len = text.len();

    while pos < text_len && mi < markup_spans.len() && hi < highlight_spans.len() {
        let m_end = markup_spans[mi].0.end.min(text_len);
        let h_end = highlight_spans[hi].byte_range.end.min(text_len);
        let end = m_end.min(h_end);

        if end > pos {
            let mstyle = &markup_spans[mi].1;
            let hkind = highlight_spans[hi].kind;
            let format = make_combined_format(
                mstyle,
                hkind,
                &default_font,
                base_font_size,
                text_color,
                code_bg,
                link_color,
                dimmed_color,
                colors,
            );
            sections.push(egui::text::LayoutSection {
                leading_space: 0.0,
                byte_range: pos..end,
                format,
            });
        }

        pos = end;
        if pos >= m_end {
            mi += 1;
        }
        if pos >= h_end {
            hi += 1;
        }
    }

    let mut job = LayoutJob {
        text: text.to_owned(),
        sections,
        break_on_newline: true,
        wrap: egui::text::TextWrapping {
            max_width: wrap_width,
            ..Default::default()
        },
        ..Default::default()
    };

    // Defensive: if no sections, add one covering everything
    if job.sections.is_empty() && !text.is_empty() {
        job.sections.push(egui::text::LayoutSection {
            leading_space: 0.0,
            byte_range: 0..text_len,
            format: TextFormat {
                font_id: default_font,
                color: text_color,
                ..Default::default()
            },
        });
    }

    job
}

/// Combine a markup style and a highlight kind into a single TextFormat.
#[allow(clippy::too_many_arguments)]
fn make_combined_format(
    mstyle: &markup::Style,
    hkind: SpanKind,
    default_font: &FontId,
    base_size: f32,
    text_color: Color32,
    code_bg: Color32,
    link_color: Color32,
    dimmed_color: Color32,
    colors: &HighlightColors,
) -> TextFormat {
    // Start with base format from markup
    let mut font = default_font.clone();
    let mut color = text_color;
    let mut background = Color32::TRANSPARENT;
    let mut italics = false;
    let mut strikethrough = Stroke::NONE;
    let mut underline = Stroke::NONE;

    // Apply markup formatting
    if mstyle.heading > 0 {
        let scale = match mstyle.heading {
            1 => 1.6,
            2 => 1.4,
            3 => 1.25,
            4 => 1.15,
            5 => 1.1,
            _ => 1.05,
        };
        font = FontId::new(base_size * scale, FontFamily::Proportional);
    }
    if mstyle.bold {
        let r = color.r().saturating_add(20);
        let g = color.g().saturating_add(20);
        let b = color.b().saturating_add(20);
        color = Color32::from_rgba_premultiplied(r, g, b, color.a());
        font.size *= 1.02;
    }
    if mstyle.italic {
        italics = true;
    }
    if mstyle.strikethrough {
        strikethrough = Stroke::new(1.0, text_color);
    }
    if mstyle.code {
        font = FontId::new(base_size, FontFamily::Monospace);
        background = code_bg;
    }
    if mstyle.quote {
        color = dimmed_color;
        italics = true;
    }
    if mstyle.url {
        color = link_color;
        underline = Stroke::new(1.0, link_color);
    }

    // Apply highlight overlay
    match hkind {
        SpanKind::Normal => {}
        SpanKind::SearchMatch => {
            background = colors.search_bg;
        }
        SpanKind::ActiveSearchMatch => {
            background = colors.active_search_bg;
        }
        SpanKind::WikiLink => {
            color = colors.link_color;
            underline = Stroke::new(1.0, colors.link_color);
        }
    }

    TextFormat {
        font_id: font,
        color,
        background,
        italics,
        strikethrough,
        underline,
        ..Default::default()
    }
}

/// Convert a byte offset in `text` to a char (grapheme-unaware) offset.
/// Panics if `byte_offset` is not on a char boundary.
pub fn byte_offset_to_char_offset(text: &str, byte_offset: usize) -> usize {
    text[..byte_offset].chars().count()
}

/// Convert a char offset to a byte offset.
/// If `char_offset` is beyond the end of the string, returns `text.len()`.
pub fn char_offset_to_byte_offset(text: &str, char_offset: usize) -> usize {
    text.char_indices()
        .nth(char_offset)
        .map(|(byte_idx, _)| byte_idx)
        .unwrap_or(text.len())
}

/// Detect if the cursor is inside an unclosed `[[` wiki link context.
///
/// Returns `Some((partial_text, start_char_offset))` where `partial_text` is the
/// text typed so far after `[[`, and `start_char_offset` is the char position
/// right after `[[`.
///
/// `cursor_char` is the cursor position in **char** offsets.
#[allow(dead_code)]
pub fn detect_link_autocomplete_context(text: &str, cursor_char: usize) -> Option<(String, usize)> {
    let cursor_byte = char_offset_to_byte_offset(text, cursor_char);
    let before_cursor = &text[..cursor_byte];

    // Search backwards for [[ without a closing ]]
    if let Some(bracket_pos) = before_cursor.rfind("[[") {
        let after_bracket = &before_cursor[bracket_pos + 2..];
        // Make sure there's no ]] between [[ and cursor
        if !after_bracket.contains("]]") && !after_bracket.contains('\n') {
            let start_char = byte_offset_to_char_offset(text, bracket_pos + 2);
            return Some((after_bracket.to_string(), start_char));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── find_search_matches ───

    #[test]
    fn test_search_matches_empty_query() {
        assert_eq!(find_search_matches("hello", ""), Vec::<Range<usize>>::new());
    }

    #[test]
    fn test_search_matches_no_match() {
        assert_eq!(
            find_search_matches("hello world", "xyz"),
            Vec::<Range<usize>>::new()
        );
    }

    #[test]
    fn test_search_matches_single() {
        let matches = find_search_matches("hello world", "world");
        assert_eq!(matches, vec![6..11]);
    }

    #[test]
    fn test_search_matches_case_insensitive() {
        let matches = find_search_matches("Hello World", "hello");
        assert_eq!(matches, vec![0..5]);
    }

    #[test]
    fn test_search_matches_multiple() {
        let matches = find_search_matches("abcabc", "abc");
        assert_eq!(matches, vec![0..3, 3..6]);
    }

    #[test]
    fn test_search_matches_overlapping() {
        // "aaa" searching for "aa" should find two overlapping matches
        let matches = find_search_matches("aaa", "aa");
        assert_eq!(matches, vec![0..2, 1..3]);
    }

    #[test]
    fn test_search_matches_unicode() {
        // "cafe\u{0301}" = "café" where e+combining accent is 2 chars, 3 bytes
        let text = "I like caf\u{00e9}s";
        let matches = find_search_matches(text, "caf\u{00e9}");
        assert_eq!(matches.len(), 1);
        // "I like " = 7 bytes, "café" = 5 bytes (caf = 3 + é = 2)
        assert_eq!(matches[0], 7..12);
    }

    #[test]
    fn test_search_matches_unicode_multibyte() {
        let text = "Hello \u{1f600} world";
        let matches = find_search_matches(text, "world");
        assert_eq!(matches.len(), 1);
        // "Hello " = 6, emoji = 4 bytes, " " = 1 => "world" starts at 11
        assert_eq!(matches[0], 11..16);
    }

    // ─── find_wiki_links ───

    #[test]
    fn test_wiki_links_none() {
        assert_eq!(find_wiki_links("hello world"), Vec::<Range<usize>>::new());
    }

    #[test]
    fn test_wiki_links_single() {
        let links = find_wiki_links("see [[note]] here");
        assert_eq!(links, vec![4..12]);
    }

    #[test]
    fn test_wiki_links_multiple() {
        let links = find_wiki_links("[[a]] and [[b]]");
        assert_eq!(links, vec![0..5, 10..15]);
    }

    #[test]
    fn test_wiki_links_empty_rejected() {
        // [[]] has nothing inside, should be rejected
        let links = find_wiki_links("empty [[]] link");
        assert_eq!(links, Vec::<Range<usize>>::new());
    }

    #[test]
    fn test_wiki_links_unclosed() {
        let links = find_wiki_links("unclosed [[link here");
        assert_eq!(links, Vec::<Range<usize>>::new());
    }

    #[test]
    fn test_wiki_links_no_newline_crossing() {
        let links = find_wiki_links("[[link\nacross lines]]");
        assert_eq!(links, Vec::<Range<usize>>::new());
    }

    #[test]
    fn test_wiki_links_adjacent() {
        let links = find_wiki_links("[[a]][[b]]");
        assert_eq!(links, vec![0..5, 5..10]);
    }

    // ─── find_spans ───

    #[test]
    fn test_spans_empty_text() {
        let spans = find_spans("", "query", None);
        assert!(spans.is_empty());
    }

    #[test]
    fn test_spans_no_matches() {
        let spans = find_spans("hello world", "", None);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].kind, SpanKind::Normal);
        assert_eq!(spans[0].byte_range, 0..11);
    }

    #[test]
    fn test_spans_search_match() {
        let spans = find_spans("hello world", "world", None);
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].kind, SpanKind::Normal);
        assert_eq!(spans[0].byte_range, 0..6);
        assert_eq!(spans[1].kind, SpanKind::SearchMatch);
        assert_eq!(spans[1].byte_range, 6..11);
    }

    #[test]
    fn test_spans_active_match() {
        let spans = find_spans("abc abc", "abc", Some(1));
        // Should be: SearchMatch(0..3), Normal(3..4), ActiveSearchMatch(4..7)
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].kind, SpanKind::SearchMatch);
        assert_eq!(spans[1].kind, SpanKind::Normal);
        assert_eq!(spans[2].kind, SpanKind::ActiveSearchMatch);
    }

    #[test]
    fn test_spans_wiki_link_overrides_search() {
        let text = "see [[hello]] world";
        let spans = find_spans(text, "hello", None);
        // Wiki link should take precedence; "hello" inside [[ ]] should NOT be a search match
        let wiki_spans: Vec<_> = spans
            .iter()
            .filter(|s| s.kind == SpanKind::WikiLink)
            .collect();
        assert_eq!(wiki_spans.len(), 1);
        let search_spans: Vec<_> = spans
            .iter()
            .filter(|s| s.kind == SpanKind::SearchMatch)
            .collect();
        assert_eq!(search_spans.len(), 0);
    }

    #[test]
    fn test_spans_full_coverage() {
        let text = "hello [[link]] world";
        let spans = find_spans(text, "world", None);
        // Verify all bytes are covered
        let total_bytes: usize = spans
            .iter()
            .map(|s| s.byte_range.end - s.byte_range.start)
            .sum();
        assert_eq!(total_bytes, text.len());
        // Verify contiguous
        for i in 1..spans.len() {
            assert_eq!(spans[i].byte_range.start, spans[i - 1].byte_range.end);
        }
    }

    // ─── build_layout_job ───

    #[test]
    fn test_layout_job_covers_all_bytes() {
        let text = "Hello \u{1f600} world [[link]]";
        let spans = find_spans(text, "world", Some(0));
        let colors = HighlightColors {
            default_text: Color32::WHITE,
            search_bg: Color32::YELLOW,
            active_search_bg: Color32::from_rgba_premultiplied(255, 165, 0, 153),
            link_color: Color32::LIGHT_BLUE,
        };
        let font_id = FontId::monospace(14.0);
        let job = build_layout_job(text, &spans, font_id, &colors, 400.0);

        assert_eq!(job.text, text);
        // Verify all bytes covered
        let total: usize = job
            .sections
            .iter()
            .map(|s| s.byte_range.end - s.byte_range.start)
            .sum();
        assert_eq!(total, text.len());
        // Verify contiguous
        for i in 1..job.sections.len() {
            assert_eq!(
                job.sections[i].byte_range.start,
                job.sections[i - 1].byte_range.end
            );
        }
    }

    #[test]
    fn test_layout_job_empty_text() {
        let spans = find_spans("", "", None);
        let colors = HighlightColors {
            default_text: Color32::WHITE,
            search_bg: Color32::YELLOW,
            active_search_bg: Color32::from_rgba_premultiplied(255, 165, 0, 153),
            link_color: Color32::LIGHT_BLUE,
        };
        let font_id = FontId::monospace(14.0);
        let job = build_layout_job("", &spans, font_id, &colors, 400.0);
        assert!(job.sections.is_empty());
    }

    #[test]
    fn test_layout_job_no_query() {
        let text = "plain text here";
        let spans = find_spans(text, "", None);
        let colors = HighlightColors {
            default_text: Color32::WHITE,
            search_bg: Color32::YELLOW,
            active_search_bg: Color32::from_rgba_premultiplied(255, 165, 0, 153),
            link_color: Color32::LIGHT_BLUE,
        };
        let font_id = FontId::monospace(14.0);
        let job = build_layout_job(text, &spans, font_id, &colors, 400.0);
        assert_eq!(job.sections.len(), 1);
        assert_eq!(job.sections[0].byte_range, 0..text.len());
    }

    // ─── byte/char conversion ───

    #[test]
    fn test_byte_to_char_ascii() {
        assert_eq!(byte_offset_to_char_offset("hello", 0), 0);
        assert_eq!(byte_offset_to_char_offset("hello", 3), 3);
        assert_eq!(byte_offset_to_char_offset("hello", 5), 5);
    }

    #[test]
    fn test_byte_to_char_unicode() {
        // "café" where é is 2 bytes
        let text = "caf\u{00e9}";
        assert_eq!(byte_offset_to_char_offset(text, 0), 0); // 'c'
        assert_eq!(byte_offset_to_char_offset(text, 3), 3); // before 'é'
        assert_eq!(byte_offset_to_char_offset(text, 5), 4); // after 'é' (end)
    }

    #[test]
    fn test_byte_to_char_emoji() {
        // emoji is 4 bytes
        let text = "a\u{1f600}b";
        assert_eq!(byte_offset_to_char_offset(text, 0), 0); // 'a'
        assert_eq!(byte_offset_to_char_offset(text, 1), 1); // start of emoji
        assert_eq!(byte_offset_to_char_offset(text, 5), 2); // 'b'
        assert_eq!(byte_offset_to_char_offset(text, 6), 3); // end
    }

    #[test]
    fn test_char_to_byte_ascii() {
        assert_eq!(char_offset_to_byte_offset("hello", 0), 0);
        assert_eq!(char_offset_to_byte_offset("hello", 3), 3);
        assert_eq!(char_offset_to_byte_offset("hello", 5), 5);
    }

    #[test]
    fn test_char_to_byte_unicode() {
        let text = "caf\u{00e9}";
        assert_eq!(char_offset_to_byte_offset(text, 0), 0);
        assert_eq!(char_offset_to_byte_offset(text, 3), 3);
        assert_eq!(char_offset_to_byte_offset(text, 4), 5); // after 'é'
    }

    #[test]
    fn test_char_to_byte_emoji() {
        let text = "a\u{1f600}b";
        assert_eq!(char_offset_to_byte_offset(text, 0), 0);
        assert_eq!(char_offset_to_byte_offset(text, 1), 1);
        assert_eq!(char_offset_to_byte_offset(text, 2), 5);
        assert_eq!(char_offset_to_byte_offset(text, 3), 6);
    }

    #[test]
    fn test_char_to_byte_beyond_end() {
        assert_eq!(char_offset_to_byte_offset("hello", 100), 5);
    }

    #[test]
    fn test_roundtrip_byte_char() {
        let text = "Hello \u{1f600} caf\u{00e9} world";
        for (byte_idx, _) in text.char_indices() {
            let char_off = byte_offset_to_char_offset(text, byte_idx);
            let back = char_offset_to_byte_offset(text, char_off);
            assert_eq!(back, byte_idx, "roundtrip failed for byte_idx={}", byte_idx);
        }
    }

    // ─── detect_link_autocomplete_context ───

    #[test]
    fn test_autocomplete_no_context() {
        assert_eq!(detect_link_autocomplete_context("hello world", 5), None);
    }

    #[test]
    fn test_autocomplete_inside_open_bracket() {
        let text = "see [[not";
        let cursor_char = 9; // at end
        let result = detect_link_autocomplete_context(text, cursor_char);
        assert_eq!(result, Some(("not".to_string(), 6)));
    }

    #[test]
    fn test_autocomplete_closed_bracket() {
        let text = "see [[note]] more";
        let cursor_char = 17; // after ]] at end
        assert_eq!(detect_link_autocomplete_context(text, cursor_char), None);
    }

    #[test]
    fn test_autocomplete_empty_partial() {
        let text = "see [[";
        let cursor_char = 6;
        let result = detect_link_autocomplete_context(text, cursor_char);
        assert_eq!(result, Some(("".to_string(), 6)));
    }

    #[test]
    fn test_autocomplete_with_newline_breaks() {
        let text = "see [[\nnote";
        let cursor_char = 11;
        // Newline between [[ and cursor should prevent match
        assert_eq!(detect_link_autocomplete_context(text, cursor_char), None);
    }

    #[test]
    fn test_autocomplete_unicode() {
        let text = "see [[caf\u{00e9}";
        let cursor_char = text.chars().count();
        let result = detect_link_autocomplete_context(text, cursor_char);
        assert_eq!(result, Some(("caf\u{00e9}".to_string(), 6)));
    }
}
