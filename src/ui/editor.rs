use crate::app::{FocusTarget, NvApp};
use crate::highlight;
use crate::shortcuts;
use crate::theme::highlight_colors;
use crate::ui::formatting;
use egui::text::CCursor;
use egui::text_selection::CCursorRange;
use egui::TextBuffer;
use std::ops::Range;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Inline wiki-link helpers (standalone, no dependency on highlight.rs)
// These can be refactored to use highlight::find_wiki_links from Stream 2 later.
// ---------------------------------------------------------------------------

/// A wiki link span: character-offset range covering `[[target]]` and the inner target text.
#[derive(Debug, Clone, PartialEq, Eq)]
struct WikiLink {
    /// Character-offset range of the full `[[...]]` (inclusive of brackets).
    full_range: Range<usize>,
    /// The inner target text (between the brackets).
    target: String,
}

/// Scan `text` for `[[...]]` wiki links. Returns spans with **character** offsets.
/// Handles edge cases: ignores empty `[[]]`, doesn't nest, and requires closing `]]`.
fn find_wiki_links(text: &str) -> Vec<WikiLink> {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut links = Vec::new();
    let mut i = 0;

    while i + 1 < len {
        if chars[i] == '[' && chars[i + 1] == '[' {
            let open = i;
            let mut j = i + 2;
            let mut found_close = false;

            while j + 1 < len {
                // If we encounter another `[[` before closing, break (no nesting).
                if chars[j] == '[' && chars[j + 1] == '[' {
                    break;
                }
                if chars[j] == ']' && chars[j + 1] == ']' {
                    let target: String = chars[open + 2..j].iter().collect();
                    if !target.is_empty() {
                        links.push(WikiLink {
                            full_range: open..j + 2,
                            target,
                        });
                    }
                    i = j + 2;
                    found_close = true;
                    break;
                }
                // Don't allow newlines inside wiki links
                if chars[j] == '\n' {
                    break;
                }
                j += 1;
            }

            if !found_close {
                i += 1;
            }
        } else {
            i += 1;
        }
    }

    links
}

/// Detect if the cursor is inside an unclosed `[[` (for autocomplete).
/// Returns `Some((partial_text, start_char_of_partial))` if cursor is inside `[[...`
/// where there is no matching `]]` yet.
/// `cursor_char` is the character offset of the cursor position.
fn detect_link_autocomplete_context(text: &str, cursor_char: usize) -> Option<(String, usize)> {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();

    // Search backwards from cursor to find `[[`
    if cursor_char < 2 || cursor_char > len {
        return None;
    }

    // Walk backwards from cursor looking for `[[`
    let mut i = cursor_char;
    while i >= 2 {
        // Check if we hit a `]]` before finding `[[` -- means we're not inside a link
        if i <= cursor_char && i >= 2 && chars[i - 1] == ']' && chars[i - 2] == ']' {
            return None;
        }
        // Don't cross newlines
        if i < cursor_char && chars[i] == '\n' {
            return None;
        }
        if chars[i - 2] == '[' && chars[i - 1] == '[' {
            // Found opening `[[` at char position i-2
            // Check there is no `]]` between i and cursor
            let partial: String = chars[i..cursor_char].iter().collect();
            // Verify no `]]` in the partial
            if partial.contains("]]") {
                return None;
            }
            return Some((partial, i));
        }
        i -= 1;
    }

    None
}

/// Find which wiki link (if any) contains the given character offset.
fn wiki_link_at_char(links: &[WikiLink], char_offset: usize) -> Option<&WikiLink> {
    links
        .iter()
        .find(|link| char_offset >= link.full_range.start && char_offset < link.full_range.end)
}

impl NvApp {
    pub fn show_editor(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(idx) = self.selected_index {
                let mut content = match self.store.get(idx) {
                    Some(note) => note.content.clone(),
                    None => return,
                };
                let title = match self.store.get(idx) {
                    Some(note) => note.title.clone(),
                    None => return,
                };

                ui.heading(&title);

                // --- Tag pills + input ---
                let tags_clone: Vec<String> = self
                    .selected_index
                    .and_then(|i| self.store.get(i))
                    .map(|n| n.tags.clone())
                    .unwrap_or_default();

                let mut tag_to_remove: Option<String> = None;
                let mut tag_to_add: Option<String> = None;

                ui.horizontal_wrapped(|ui| {
                    // Render existing tags as pills
                    for tag in &tags_clone {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 2.0;
                            let pill = egui::Frame::NONE
                                .fill(ui.visuals().widgets.inactive.bg_fill)
                                .corner_radius(egui::CornerRadius::same(3))
                                .inner_margin(egui::Margin::symmetric(6, 2));
                            pill.show(ui, |ui| {
                                ui.label(egui::RichText::new(tag).small());
                                if ui
                                    .small_button("\u{00d7}")
                                    .on_hover_text("Remove tag")
                                    .clicked()
                                {
                                    tag_to_remove = Some(tag.clone());
                                }
                            });
                        });
                    }

                    // Tag input field
                    let tag_input_response = ui.add_sized(
                        [100.0, 18.0],
                        egui::TextEdit::singleline(&mut self.tag_input)
                            .hint_text("Add tag...")
                            .font(egui::TextStyle::Small),
                    );

                    if self.focus == FocusTarget::Tags {
                        tag_input_response.request_focus();
                        self.focus = FocusTarget::None;
                    }

                    // Commit tag on Enter
                    if tag_input_response.lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    {
                        let tag = self.tag_input.trim().to_string();
                        if !tag.is_empty() {
                            tag_to_add = Some(tag);
                        }
                    }

                    // Update suggestions when input changes
                    if tag_input_response.changed() {
                        self.update_tag_suggestions();
                    }

                    // Tag autocomplete popup
                    if self.show_tag_suggestions && !self.tag_suggestions.is_empty() {
                        let popup_id = ui.id().with("tag_autocomplete");
                        let popup_pos = tag_input_response.rect.left_bottom();

                        egui::Area::new(popup_id)
                            .fixed_pos(popup_pos)
                            .order(egui::Order::Foreground)
                            .show(ui.ctx(), |ui| {
                                egui::Frame::popup(ui.style()).show(ui, |ui| {
                                    let suggestions = self.tag_suggestions.clone();
                                    for suggestion in &suggestions {
                                        if ui.selectable_label(false, suggestion).clicked() {
                                            tag_to_add = Some(suggestion.clone());
                                        }
                                    }
                                });
                            });
                    }
                });

                // Apply tag changes
                if let Some(tag) = tag_to_remove {
                    self.remove_tag_from_selected(&tag);
                }
                if let Some(tag) = tag_to_add {
                    self.add_tag_to_selected(&tag);
                }

                ui.separator();

                // Apply pending wiki link autocomplete before showing TextEdit.
                // This splices the replacement into content so the TextEdit sees it.
                let pending_cursor =
                    if let Some((start, end, ref replacement)) = self.pending_completion.take() {
                        let chars: Vec<char> = content.chars().collect();
                        let before: String = chars[..start].iter().collect();
                        let after: String = if end <= chars.len() {
                            chars[end..].iter().collect()
                        } else {
                            String::new()
                        };
                        content = format!("{}{}{}", before, replacement, after);
                        // Place cursor after the replacement
                        Some(start + replacement.chars().count())
                    } else {
                        None
                    };

                let content_len = content.chars().count();

                // Track whether Cmd/Ctrl is held for link clicking
                let cmd_held = ui.input(|i| i.modifiers.command);

                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        let editor_font_size = self.settings.editor_font_size;

                        // Prepare data for the combined layouter closure
                        let search_query = self.search_query.clone();
                        let active_match = self.find_match_index;
                        let colors = highlight_colors(self.settings.theme);

                        let mut layouter = |ui: &egui::Ui,
                                            buf: &dyn TextBuffer,
                                            wrap_width: f32|
                         -> Arc<egui::text::Galley> {
                            let text = buf.as_str();
                            let job = highlight::build_combined_layout_job(
                                text,
                                &search_query,
                                active_match,
                                editor_font_size,
                                ui.style(),
                                &colors,
                                wrap_width,
                            );
                            ui.fonts_mut(|f| f.layout_job(job))
                        };

                        let output = egui::TextEdit::multiline(&mut content)
                            .desired_width(f32::INFINITY)
                            .desired_rows(20)
                            .layouter(&mut layouter)
                            .show(ui);

                        // Update find_match_count from the current content
                        let matches = highlight::find_search_matches(&content, &self.search_query);
                        self.find_match_count = matches.len();

                        // Apply pending cursor position from find next/prev
                        if let Some(char_offset) = self.pending_cursor_move.take() {
                            let mut state = output.state.clone();
                            state
                                .cursor
                                .set_char_range(Some(CCursorRange::one(CCursor::new(char_offset))));
                            state.store(ui.ctx(), output.response.id);
                        }

                        // Apply pending cursor position from wiki link autocomplete
                        if let Some(char_offset) = pending_cursor {
                            let mut state = output.state.clone();
                            state
                                .cursor
                                .set_char_range(Some(CCursorRange::one(CCursor::new(char_offset))));
                            state.store(ui.ctx(), output.response.id);
                        }

                        if self.focus == FocusTarget::Editor {
                            output.response.request_focus();
                            self.focus = FocusTarget::None;
                        }

                        // --- Wiki link: Cmd+click to follow ---
                        let wiki_links = find_wiki_links(&content);

                        if cmd_held && output.response.clicked() {
                            // Determine character offset at click position
                            if let Some(cursor_range) = output.cursor_range {
                                let click_char = cursor_range.primary.index;
                                if let Some(link) = wiki_link_at_char(&wiki_links, click_char) {
                                    let target = link.target.clone();
                                    // Defer navigation (will happen after content update)
                                    // We need to update content first, then navigate
                                    if let Some(note) = self.store.get_mut(idx) {
                                        if note.content != content {
                                            note.update_content(content.clone());
                                        }
                                    }
                                    self.navigate_to_link(&target);
                                    return;
                                }
                            }
                        }

                        // --- Wiki link: hand cursor when Cmd + hovering over link ---
                        if cmd_held && output.response.hovered() {
                            if let Some(pointer_pos) = ui.ctx().pointer_latest_pos() {
                                // Try to determine if pointer is over a wiki link
                                // Use the galley to convert pointer position to cursor
                                let galley = &output.galley;
                                let galley_pos = output.galley_pos;
                                let relative_pos = pointer_pos - galley_pos;
                                let cursor = galley.cursor_from_pos(relative_pos);
                                let hover_char = cursor.index;
                                if wiki_link_at_char(&wiki_links, hover_char).is_some() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                }
                            }
                        }

                        // --- Wiki link autocomplete popup ---
                        if let Some(cursor_range) = output.cursor_range {
                            let cursor_char = cursor_range.primary.index;
                            if let Some((partial, start_char)) =
                                detect_link_autocomplete_context(&content, cursor_char)
                            {
                                // Collect matching note titles
                                let partial_lower = partial.to_lowercase();
                                let suggestions: Vec<String> = self
                                    .store
                                    .notes
                                    .iter()
                                    .filter(|n| {
                                        n.title_lower.contains(&partial_lower) && n.title != title
                                    })
                                    .map(|n| n.title.clone())
                                    .take(8)
                                    .collect();

                                if !suggestions.is_empty() {
                                    // Position the popup near the cursor
                                    let popup_id = ui.id().with("wiki_link_autocomplete");

                                    // Calculate position from galley cursor
                                    let galley = &output.galley;
                                    let galley_pos = output.galley_pos;
                                    let cursor_pos =
                                        galley.pos_from_cursor(CCursor::new(cursor_char));
                                    let popup_pos = galley_pos
                                        + egui::vec2(cursor_pos.min.x, cursor_pos.max.y + 4.0);

                                    egui::Area::new(popup_id)
                                        .fixed_pos(popup_pos)
                                        .order(egui::Order::Foreground)
                                        .show(ui.ctx(), |ui| {
                                            egui::Frame::popup(ui.style()).show(ui, |ui| {
                                                for suggestion in &suggestions {
                                                    if ui
                                                        .selectable_label(false, suggestion)
                                                        .clicked()
                                                    {
                                                        // Build replacement: `target]]`
                                                        // start_char points to after `[[`, we replace
                                                        // from start_char to cursor_char with `target]]`
                                                        let replacement =
                                                            format!("{}]]", suggestion);
                                                        self.pending_completion = Some((
                                                            start_char,
                                                            cursor_char,
                                                            replacement,
                                                        ));
                                                    }
                                                }
                                            });
                                        });
                                }
                            }
                        }

                        // Get selection range for context menu actions
                        let selection = output.cursor_range.map(|cr| {
                            let range = cr.as_sorted_char_range();
                            (range.start, range.end)
                        });
                        let has_selection = selection.is_some_and(|(start, end)| start != end);

                        // --- Formatting shortcuts (Cmd+B/I/etc) ---
                        {
                            let fmt_action: Option<&str> = ui.input_mut(|input| {
                                if input.consume_shortcut(&shortcuts::BOLD) {
                                    return Some("**");
                                }
                                if input.consume_shortcut(&shortcuts::ITALIC) {
                                    return Some("*");
                                }
                                if input.consume_shortcut(&shortcuts::STRIKETHROUGH) {
                                    return Some("~~");
                                }
                                if input.consume_shortcut(&shortcuts::CODE) {
                                    return Some("`");
                                }
                                if input.consume_shortcut(&shortcuts::INDENT) {
                                    return Some("__INDENT__");
                                }
                                if input.consume_shortcut(&shortcuts::OUTDENT) {
                                    return Some("__OUTDENT__");
                                }
                                None
                            });

                            if let Some(action) = fmt_action {
                                if let Some(cursor_range) = output.cursor_range {
                                    let range = cursor_range.as_sorted_char_range();
                                    let (new_content, new_start, new_end) = match action {
                                        "__INDENT__" => formatting::indent_lines(
                                            &content,
                                            range.start,
                                            range.end,
                                        ),
                                        "__OUTDENT__" => formatting::outdent_lines(
                                            &content,
                                            range.start,
                                            range.end,
                                        ),
                                        delimiter => formatting::toggle_surrounding(
                                            &content,
                                            range.start,
                                            range.end,
                                            delimiter,
                                        ),
                                    };
                                    content = new_content;
                                    let mut state = output.state.clone();
                                    state.cursor.set_char_range(Some(CCursorRange::two(
                                        CCursor::new(new_start),
                                        CCursor::new(new_end),
                                    )));
                                    state.store(ui.ctx(), output.response.id);
                                }
                            }
                        }

                        // --- URL Cmd+click to open ---
                        if cmd_held && output.response.clicked() {
                            if let Some(cursor_range) = output.cursor_range {
                                let click_char = cursor_range.primary.index;
                                let click_byte =
                                    highlight::char_offset_to_byte_offset(&content, click_char);
                                let urls = formatting::find_urls(&content);
                                for url_range in &urls {
                                    if url_range.contains(&click_byte) {
                                        let url = &content[url_range.clone()];
                                        let _ = opener::open(url);
                                        break;
                                    }
                                }
                            }
                        }

                        output.response.context_menu(|ui| {
                            let cut_btn = ui.add_enabled(has_selection, egui::Button::new("Cut"));
                            if cut_btn.clicked() {
                                if let Some((start, end)) = selection {
                                    let selected: String =
                                        content.chars().skip(start).take(end - start).collect();
                                    NvApp::copy_to_clipboard(&selected);
                                    let before: String = content.chars().take(start).collect();
                                    let after: String = content.chars().skip(end).collect();
                                    content = format!("{}{}", before, after);
                                }
                                ui.close();
                            }

                            let copy_btn = ui.add_enabled(has_selection, egui::Button::new("Copy"));
                            if copy_btn.clicked() {
                                if let Some((start, end)) = selection {
                                    let selected: String =
                                        content.chars().skip(start).take(end - start).collect();
                                    NvApp::copy_to_clipboard(&selected);
                                }
                                ui.close();
                            }

                            if ui.button("Paste").clicked() {
                                if let Ok(mut cb) = arboard::Clipboard::new() {
                                    if let Ok(clip_text) = cb.get_text() {
                                        if let Some((start, end)) = selection {
                                            let before: String =
                                                content.chars().take(start).collect();
                                            let after: String = content.chars().skip(end).collect();
                                            content = format!("{}{}{}", before, clip_text, after);
                                        } else if let Some(cr) = output.cursor_range {
                                            let pos = cr.primary.index;
                                            let before: String =
                                                content.chars().take(pos).collect();
                                            let after: String = content.chars().skip(pos).collect();
                                            content = format!("{}{}{}", before, clip_text, after);
                                        } else {
                                            content.push_str(&clip_text);
                                        }
                                    }
                                }
                                ui.close();
                            }

                            ui.separator();

                            if ui.button("Select All").clicked() {
                                let mut state = output.state.clone();
                                state.cursor.set_char_range(Some(CCursorRange::two(
                                    CCursor::new(0),
                                    CCursor::new(content_len),
                                )));
                                state.store(ui.ctx(), output.response.id);
                                ui.close();
                            }
                        });
                    });

                // Auto-indent: if a newline was just typed, apply indentation
                if self.settings.auto_indent {
                    if let Some(cursor_range) =
                        egui::TextEdit::load_state(ctx, egui::Id::new("editor_text"))
                            .and_then(|s| s.cursor.char_range())
                    {
                        let cursor_char = cursor_range.primary.index;
                        if let Some((new_content, new_cursor)) =
                            formatting::auto_indent(&content, cursor_char)
                        {
                            content = new_content;
                            // Note: cursor update will happen on next frame
                            self.pending_cursor_move = Some(new_cursor);
                        }
                    }
                }

                // Update note content if changed
                if let Some(note) = self.store.get_mut(idx) {
                    if note.content != content {
                        note.update_content(content);
                    }
                }
            } else {
                ui.centered_and_justified(|ui| {
                    ui.weak("Select or create a note to begin");
                });
            }
        });
    }
}

// ---------------------------------------------------------------------------
// Tests for inline wiki-link helpers
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- find_wiki_links ---

    #[test]
    fn test_find_wiki_links_basic() {
        let links = find_wiki_links("Hello [[World]] there");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "World");
        assert_eq!(links[0].full_range, 6..15);
    }

    #[test]
    fn test_find_wiki_links_multiple() {
        let links = find_wiki_links("See [[Alpha]] and [[Beta]]");
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].target, "Alpha");
        assert_eq!(links[1].target, "Beta");
    }

    #[test]
    fn test_find_wiki_links_empty_brackets() {
        let links = find_wiki_links("Empty [[]] link");
        assert_eq!(links.len(), 0);
    }

    #[test]
    fn test_find_wiki_links_unclosed() {
        let links = find_wiki_links("Unclosed [[ link here");
        assert_eq!(links.len(), 0);
    }

    #[test]
    fn test_find_wiki_links_no_nesting() {
        // Nested [[inner]] should not match the outer
        let links = find_wiki_links("[[outer [[inner]]]]");
        // The parser should find "inner" since it encounters [[ inside and restarts
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "inner");
    }

    #[test]
    fn test_find_wiki_links_close_without_open() {
        let links = find_wiki_links("Just ]] without open");
        assert_eq!(links.len(), 0);
    }

    #[test]
    fn test_find_wiki_links_newline_breaks_link() {
        let links = find_wiki_links("[[broken\nlink]]");
        assert_eq!(links.len(), 0);
    }

    #[test]
    fn test_find_wiki_links_adjacent() {
        let links = find_wiki_links("[[A]][[B]]");
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].target, "A");
        assert_eq!(links[1].target, "B");
    }

    #[test]
    fn test_find_wiki_links_unicode() {
        let links = find_wiki_links("Link to [[cafe\u{0301}]]");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "cafe\u{0301}");
    }

    #[test]
    fn test_find_wiki_links_spaces_in_target() {
        let links = find_wiki_links("[[My Note Title]]");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "My Note Title");
    }

    // --- detect_link_autocomplete_context ---

    #[test]
    fn test_autocomplete_inside_unclosed_link() {
        let text = "Hello [[wor";
        let cursor = 11; // after 'r'
        let result = detect_link_autocomplete_context(text, cursor);
        assert!(result.is_some());
        let (partial, start) = result.unwrap();
        assert_eq!(partial, "wor");
        assert_eq!(start, 8); // char position after `[[`
    }

    #[test]
    fn test_autocomplete_not_inside_link() {
        let text = "Hello world";
        let result = detect_link_autocomplete_context(text, 5);
        assert!(result.is_none());
    }

    #[test]
    fn test_autocomplete_after_closed_link() {
        let text = "Hello [[World]] more";
        // Cursor at position 18, after the closed link
        let result = detect_link_autocomplete_context(text, 18);
        assert!(result.is_none());
    }

    #[test]
    fn test_autocomplete_empty_partial() {
        let text = "Hello [[";
        let cursor = 8; // right after `[[`
        let result = detect_link_autocomplete_context(text, cursor);
        assert!(result.is_some());
        let (partial, _start) = result.unwrap();
        assert_eq!(partial, "");
    }

    #[test]
    fn test_autocomplete_cursor_at_start() {
        let text = "[[test";
        let result = detect_link_autocomplete_context(text, 0);
        assert!(result.is_none());
    }

    // --- wiki_link_at_char ---

    #[test]
    fn test_wiki_link_at_char_inside() {
        let links = find_wiki_links("Hello [[World]] there");
        // Char 8 is 'o' in World, inside the link
        assert!(wiki_link_at_char(&links, 8).is_some());
        assert_eq!(wiki_link_at_char(&links, 8).unwrap().target, "World");
    }

    #[test]
    fn test_wiki_link_at_char_on_bracket() {
        let links = find_wiki_links("Hello [[World]] there");
        // Char 6 is first `[` — should be inside the full_range
        assert!(wiki_link_at_char(&links, 6).is_some());
    }

    #[test]
    fn test_wiki_link_at_char_outside() {
        let links = find_wiki_links("Hello [[World]] there");
        // Char 0 is 'H', outside
        assert!(wiki_link_at_char(&links, 0).is_none());
        // Char 15 is ' ', right after the link
        assert!(wiki_link_at_char(&links, 15).is_none());
    }
}
