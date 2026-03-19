use std::fs;
use std::path::Path;

/// The result of importing a file: a title and markdown-formatted content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedNote {
    pub title: String,
    pub content: String,
}

/// Import a file, dispatching by extension.
/// Supported: .txt, .md, .html, .htm, .csv
pub fn import_file(path: &Path) -> Result<ImportedNote, String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "txt" | "md" => import_text(path),
        "html" | "htm" => import_html(path),
        "csv" => import_csv(path),
        _ => Err(format!(
            "Unsupported file format: .{}. Supported: .txt, .md, .html, .htm, .csv",
            ext
        )),
    }
}

/// Import a plain text or markdown file as-is.
pub fn import_text(path: &Path) -> Result<ImportedNote, String> {
    let title = title_from_path(path);
    let content =
        fs::read_to_string(path).map_err(|e| format!("Failed to read {:?}: {}", path, e))?;
    Ok(ImportedNote { title, content })
}

/// Import an HTML file by converting basic tags to markdown.
pub fn import_html(path: &Path) -> Result<ImportedNote, String> {
    let title = title_from_path(path);
    let raw = fs::read_to_string(path).map_err(|e| format!("Failed to read {:?}: {}", path, e))?;
    let content = html_to_markdown(&raw);
    Ok(ImportedNote { title, content })
}

/// Import a CSV file by converting it to a markdown table.
pub fn import_csv(path: &Path) -> Result<ImportedNote, String> {
    let title = title_from_path(path);
    let raw = fs::read_to_string(path).map_err(|e| format!("Failed to read {:?}: {}", path, e))?;
    let content = csv_to_markdown_table(&raw);
    Ok(ImportedNote { title, content })
}

/// Extract a title from a file path (filename without extension).
fn title_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Imported Note")
        .to_string()
}

// ---------------------------------------------------------------------------
// HTML-to-Markdown converter (hand-rolled, handles basic tags)
// ---------------------------------------------------------------------------

/// Convert basic HTML to markdown. Handles:
/// b, strong, i, em, p, br, h1-h6, a, ul, ol, li, and strips other tags.
pub fn html_to_markdown(html: &str) -> String {
    // First, extract the <body> content if present, otherwise use the whole thing
    let body = extract_body(html);
    let mut result = String::new();
    let mut chars = body.chars().peekable();

    while let Some(&ch) = chars.peek() {
        if ch == '<' {
            // Parse the tag
            let tag_str = consume_tag(&mut chars);
            let tag_str_trimmed = tag_str.trim().to_string();

            if tag_str_trimmed.is_empty() {
                continue;
            }

            let is_closing = tag_str_trimmed.starts_with('/');
            let tag_name = extract_tag_name(&tag_str_trimmed).to_lowercase();

            match tag_name.as_str() {
                "b" | "strong" => {
                    result.push_str("**");
                }
                "i" | "em" => {
                    result.push('*');
                }
                "br" => {
                    result.push('\n');
                }
                "p" => {
                    if is_closing {
                        result.push_str("\n\n");
                    }
                }
                "h1" => {
                    if !is_closing {
                        ensure_newline(&mut result);
                        result.push_str("# ");
                    } else {
                        result.push('\n');
                    }
                }
                "h2" => {
                    if !is_closing {
                        ensure_newline(&mut result);
                        result.push_str("## ");
                    } else {
                        result.push('\n');
                    }
                }
                "h3" => {
                    if !is_closing {
                        ensure_newline(&mut result);
                        result.push_str("### ");
                    } else {
                        result.push('\n');
                    }
                }
                "h4" => {
                    if !is_closing {
                        ensure_newline(&mut result);
                        result.push_str("#### ");
                    } else {
                        result.push('\n');
                    }
                }
                "h5" => {
                    if !is_closing {
                        ensure_newline(&mut result);
                        result.push_str("##### ");
                    } else {
                        result.push('\n');
                    }
                }
                "h6" => {
                    if !is_closing {
                        ensure_newline(&mut result);
                        result.push_str("###### ");
                    } else {
                        result.push('\n');
                    }
                }
                "a" => {
                    if !is_closing {
                        // Extract href from the tag
                        let href = extract_attribute(&tag_str_trimmed, "href");
                        if let Some(url) = href {
                            // We need to collect the link text until </a>
                            let link_text = consume_until_close_tag(&mut chars, "a");
                            result.push_str(&format!("[{}]({})", link_text, url));
                        }
                        // If no href, just include the text content (strip tag)
                    }
                    // Closing </a> is consumed by consume_until_close_tag, or ignored
                }
                "ul" | "ol" => {
                    if !is_closing {
                        ensure_newline(&mut result);
                    } else {
                        result.push('\n');
                    }
                }
                "li" => {
                    if !is_closing {
                        ensure_newline(&mut result);
                        result.push_str("- ");
                    } else {
                        result.push('\n');
                    }
                }
                "script" | "style" => {
                    if !is_closing {
                        // Skip content until closing tag
                        let _ = consume_until_close_tag(&mut chars, &tag_name);
                    }
                }
                _ => {
                    // Strip unknown tags
                }
            }
        } else if ch == '&' {
            // Handle common HTML entities
            let entity = consume_entity(&mut chars);
            result.push_str(&decode_entity(&entity));
        } else {
            result.push(ch);
            chars.next();
        }
    }

    // Clean up: collapse excessive blank lines
    clean_markdown(&result)
}

/// Extract the body content from HTML, or return the whole string if no body tag.
fn extract_body(html: &str) -> String {
    let lower = html.to_lowercase();
    if let Some(body_start) = lower.find("<body") {
        // Find the end of the opening body tag
        let after_open = html[body_start..].find('>').map(|i| body_start + i + 1);
        let body_end = lower.find("</body>").unwrap_or(html.len());
        if let Some(start) = after_open {
            return html[start..body_end].to_string();
        }
    }
    html.to_string()
}

/// Consume characters until `>` is found, returning the tag content (without < >).
fn consume_tag(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    // Skip the '<'
    chars.next();
    let mut tag = String::new();
    for ch in chars.by_ref() {
        if ch == '>' {
            break;
        }
        tag.push(ch);
    }
    tag
}

/// Consume characters until a closing tag `</name>` is found.
/// Returns the text content between the tags.
fn consume_until_close_tag(
    chars: &mut std::iter::Peekable<std::str::Chars>,
    tag_name: &str,
) -> String {
    let mut content = String::new();

    while let Some(&ch) = chars.peek() {
        if ch == '<' {
            // Check if this is the closing tag
            chars.next(); // consume '<'
            let mut tag_buf = String::new();
            for c in chars.by_ref() {
                if c == '>' {
                    break;
                }
                tag_buf.push(c);
            }
            let name = extract_tag_name(tag_buf.trim()).to_lowercase();
            if tag_buf.trim().to_lowercase().starts_with('/') && name == tag_name.to_lowercase() {
                break;
            }
            // Not our closing tag, skip it
        } else {
            content.push(ch);
            chars.next();
        }
    }

    content
}

/// Extract the tag name from a tag string like "a href='...'" -> "a" or "/p" -> "p".
fn extract_tag_name(tag: &str) -> &str {
    let s = tag.trim_start_matches('/').trim();
    // Tag name ends at first space, '/', or '>'
    let end = s
        .find(|c: char| c.is_whitespace() || c == '/' || c == '>')
        .unwrap_or(s.len());
    &s[..end]
}

/// Extract an attribute value from a tag string.
/// e.g. extract_attribute("a href=\"https://example.com\" class=\"x\"", "href") -> Some("https://example.com")
fn extract_attribute(tag: &str, attr_name: &str) -> Option<String> {
    let lower = tag.to_lowercase();
    let search = format!("{}=", attr_name.to_lowercase());
    if let Some(pos) = lower.find(&search) {
        let after_eq = &tag[pos + search.len()..];
        let after_eq = after_eq.trim_start();
        if let Some(content) = after_eq.strip_prefix('"') {
            if let Some(end) = content.find('"') {
                return Some(content[..end].to_string());
            }
        } else if let Some(content) = after_eq.strip_prefix('\'') {
            if let Some(end) = content.find('\'') {
                return Some(content[..end].to_string());
            }
        } else {
            // Unquoted attribute value
            let end = after_eq
                .find(|c: char| c.is_whitespace() || c == '>')
                .unwrap_or(after_eq.len());
            return Some(after_eq[..end].to_string());
        }
    }
    None
}

/// Consume an HTML entity starting from '&'.
fn consume_entity(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    chars.next(); // consume '&'
    let mut entity = String::from("&");
    for ch in chars.by_ref() {
        entity.push(ch);
        if ch == ';' || entity.len() > 10 {
            break;
        }
    }
    entity
}

/// Decode common HTML entities.
fn decode_entity(entity: &str) -> String {
    match entity {
        "&amp;" => "&".to_string(),
        "&lt;" => "<".to_string(),
        "&gt;" => ">".to_string(),
        "&quot;" => "\"".to_string(),
        "&apos;" => "'".to_string(),
        "&nbsp;" => " ".to_string(),
        "&mdash;" => "\u{2014}".to_string(),
        "&ndash;" => "\u{2013}".to_string(),
        "&hellip;" => "\u{2026}".to_string(),
        _ => entity.to_string(),
    }
}

/// Ensure the result string ends with a newline (don't add double newlines).
fn ensure_newline(s: &mut String) {
    if !s.is_empty() && !s.ends_with('\n') {
        s.push('\n');
    }
}

/// Clean up markdown: collapse multiple blank lines, trim trailing whitespace.
fn clean_markdown(s: &str) -> String {
    let mut result = String::new();
    let mut blank_count = 0;

    for line in s.lines() {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            blank_count += 1;
            if blank_count <= 2 {
                result.push('\n');
            }
        } else {
            blank_count = 0;
            if !result.is_empty() && !result.ends_with('\n') {
                result.push('\n');
            }
            result.push_str(trimmed);
            result.push('\n');
        }
    }

    result.trim().to_string()
}

// ---------------------------------------------------------------------------
// CSV-to-Markdown table converter (hand-rolled)
// ---------------------------------------------------------------------------

/// Convert CSV content to a markdown table.
pub fn csv_to_markdown_table(csv: &str) -> String {
    let rows: Vec<Vec<String>> = csv
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(parse_csv_line)
        .collect();

    if rows.is_empty() {
        return String::new();
    }

    // Determine column count from the header row
    let col_count = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    if col_count == 0 {
        return String::new();
    }

    let mut result = String::new();

    // Header row
    result.push('|');
    for col in 0..col_count {
        let cell = rows[0].get(col).map(|s| s.as_str()).unwrap_or("");
        result.push_str(&format!(" {} |", cell.trim()));
    }
    result.push('\n');

    // Separator row
    result.push('|');
    for _ in 0..col_count {
        result.push_str(" --- |");
    }
    result.push('\n');

    // Data rows
    for row in rows.iter().skip(1) {
        result.push('|');
        for col in 0..col_count {
            let cell = row.get(col).map(|s| s.as_str()).unwrap_or("");
            result.push_str(&format!(" {} |", cell.trim()));
        }
        result.push('\n');
    }

    result.trim_end().to_string()
}

/// Parse a single CSV line, handling quoted fields.
fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if in_quotes {
            if ch == '"' {
                // Check for escaped quote ""
                if chars.peek() == Some(&'"') {
                    current.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            } else {
                current.push(ch);
            }
        } else if ch == '"' {
            in_quotes = true;
        } else if ch == ',' {
            fields.push(current.clone());
            current.clear();
        } else {
            current.push(ch);
        }
    }

    fields.push(current);
    fields
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- HTML to Markdown ---

    #[test]
    fn test_html_bold_and_italic() {
        let html = "<b>bold</b> and <i>italic</i>";
        let md = html_to_markdown(html);
        assert!(md.contains("**bold**"));
        assert!(md.contains("*italic*"));
    }

    #[test]
    fn test_html_strong_and_em() {
        let html = "<strong>strong</strong> <em>emphasis</em>";
        let md = html_to_markdown(html);
        assert!(md.contains("**strong**"));
        assert!(md.contains("*emphasis*"));
    }

    #[test]
    fn test_html_paragraphs() {
        let html = "<p>First paragraph</p><p>Second paragraph</p>";
        let md = html_to_markdown(html);
        assert!(md.contains("First paragraph"));
        assert!(md.contains("Second paragraph"));
        // Should have blank line between paragraphs
        assert!(md.contains("\n\n"));
    }

    #[test]
    fn test_html_br_tag() {
        let html = "Line one<br>Line two";
        let md = html_to_markdown(html);
        assert!(md.contains("Line one\nLine two"));
    }

    #[test]
    fn test_html_headings() {
        let html = "<h1>Title</h1><h2>Subtitle</h2><h3>Section</h3>";
        let md = html_to_markdown(html);
        assert!(md.contains("# Title"));
        assert!(md.contains("## Subtitle"));
        assert!(md.contains("### Section"));
    }

    #[test]
    fn test_html_links() {
        let html = r#"Visit <a href="https://example.com">Example</a> here"#;
        let md = html_to_markdown(html);
        assert!(md.contains("[Example](https://example.com)"));
    }

    #[test]
    fn test_html_list() {
        let html = "<ul><li>First</li><li>Second</li></ul>";
        let md = html_to_markdown(html);
        assert!(md.contains("- First"));
        assert!(md.contains("- Second"));
    }

    #[test]
    fn test_html_entities() {
        let html = "&amp; &lt; &gt; &quot; &nbsp;";
        let md = html_to_markdown(html);
        assert!(md.contains("&"));
        assert!(md.contains("<"));
        assert!(md.contains(">"));
    }

    #[test]
    fn test_html_strips_unknown_tags() {
        let html = "<div><span>Hello</span></div>";
        let md = html_to_markdown(html);
        assert!(md.contains("Hello"));
        assert!(!md.contains("<div>"));
        assert!(!md.contains("<span>"));
    }

    #[test]
    fn test_html_strips_script_and_style() {
        let html = "<p>Hello</p><script>alert('x')</script><p>World</p>";
        let md = html_to_markdown(html);
        assert!(md.contains("Hello"));
        assert!(md.contains("World"));
        assert!(!md.contains("alert"));
    }

    #[test]
    fn test_html_body_extraction() {
        let html = "<html><head><title>T</title></head><body><p>Content</p></body></html>";
        let md = html_to_markdown(html);
        assert!(md.contains("Content"));
        assert!(!md.contains("<title>"));
    }

    // --- CSV to Markdown Table ---

    #[test]
    fn test_csv_basic_table() {
        let csv = "Name,Age,City\nAlice,30,NYC\nBob,25,LA";
        let md = csv_to_markdown_table(csv);
        assert!(md.contains("| Name | Age | City |"));
        assert!(md.contains("| --- | --- | --- |"));
        assert!(md.contains("| Alice | 30 | NYC |"));
        assert!(md.contains("| Bob | 25 | LA |"));
    }

    #[test]
    fn test_csv_quoted_fields() {
        let csv = "Name,Description\nSmith,\"A quoted value\"";
        let md = csv_to_markdown_table(csv);
        assert!(md.contains("Smith"));
        assert!(md.contains("A quoted value"));
    }

    #[test]
    fn test_csv_empty_input() {
        let md = csv_to_markdown_table("");
        assert!(md.is_empty());
    }

    #[test]
    fn test_csv_single_column() {
        let csv = "Items\nApple\nBanana";
        let md = csv_to_markdown_table(csv);
        assert!(md.contains("| Items |"));
        assert!(md.contains("| Apple |"));
    }

    // --- parse_csv_line ---

    #[test]
    fn test_parse_csv_line_simple() {
        let fields = parse_csv_line("a,b,c");
        assert_eq!(fields, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_parse_csv_line_quoted() {
        let fields = parse_csv_line(r#""hello, world",simple,"with ""quotes"""#);
        assert_eq!(fields, vec!["hello, world", "simple", "with \"quotes\""]);
    }

    // --- Import dispatch ---

    #[test]
    fn test_import_file_unsupported_extension() {
        let path = Path::new("test.docx");
        let result = import_file(path);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unsupported"));
    }

    #[test]
    fn test_import_text_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("my_note.txt");
        fs::write(&path, "Hello world").unwrap();

        let result = import_file(&path);
        assert!(result.is_ok());
        let note = result.unwrap();
        assert_eq!(note.title, "my_note");
        assert_eq!(note.content, "Hello world");
    }

    #[test]
    fn test_import_md_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("readme.md");
        fs::write(&path, "# Hello\n\nSome content").unwrap();

        let result = import_file(&path);
        assert!(result.is_ok());
        let note = result.unwrap();
        assert_eq!(note.title, "readme");
        assert_eq!(note.content, "# Hello\n\nSome content");
    }

    #[test]
    fn test_import_html_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("page.html");
        fs::write(&path, "<p><b>Hello</b> world</p>").unwrap();

        let result = import_file(&path);
        assert!(result.is_ok());
        let note = result.unwrap();
        assert_eq!(note.title, "page");
        assert!(note.content.contains("**Hello**"));
        assert!(note.content.contains("world"));
    }

    #[test]
    fn test_import_csv_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.csv");
        fs::write(&path, "A,B\n1,2").unwrap();

        let result = import_file(&path);
        assert!(result.is_ok());
        let note = result.unwrap();
        assert_eq!(note.title, "data");
        assert!(note.content.contains("| A | B |"));
    }

    // --- title_from_path ---

    #[test]
    fn test_title_from_path_normal() {
        assert_eq!(title_from_path(Path::new("notes/hello.txt")), "hello");
    }

    #[test]
    fn test_title_from_path_no_extension() {
        assert_eq!(title_from_path(Path::new("notes/hello")), "hello");
    }
}
