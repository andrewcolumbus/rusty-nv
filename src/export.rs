use pulldown_cmark::{html, Options, Parser};
use std::fs;
use std::path::Path;

/// Supported export formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    PlainText,
    Html,
    Markdown,
}

#[allow(dead_code)]
impl ExportFormat {
    /// File extension for this format.
    pub fn extension(&self) -> &'static str {
        match self {
            ExportFormat::PlainText => "txt",
            ExportFormat::Html => "html",
            ExportFormat::Markdown => "md",
        }
    }

    /// Display label for this format.
    pub fn label(&self) -> &'static str {
        match self {
            ExportFormat::PlainText => "Plain Text (.txt)",
            ExportFormat::Html => "HTML (.html)",
            ExportFormat::Markdown => "Markdown (.md)",
        }
    }
}

/// Export a note to a file in the given format.
pub fn export_note(
    title: &str,
    content: &str,
    format: ExportFormat,
    dest: &Path,
) -> Result<(), String> {
    let output = match format {
        ExportFormat::PlainText => content.to_string(),
        ExportFormat::Markdown => content.to_string(),
        ExportFormat::Html => export_to_html(title, content),
    };

    fs::write(dest, output).map_err(|e| format!("Failed to write {:?}: {}", dest, e))
}

/// Convert markdown content to a full self-contained HTML document with print-friendly CSS.
pub fn export_to_html(title: &str, content: &str) -> String {
    let options =
        Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS;
    let parser = Parser::new_ext(content, options);

    let mut body_html = String::new();
    html::push_html(&mut body_html, parser);

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title}</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
            max-width: 800px;
            margin: 0 auto;
            padding: 40px 20px;
            line-height: 1.6;
            color: #333;
        }}
        h1 {{
            border-bottom: 2px solid #eee;
            padding-bottom: 10px;
        }}
        h2 {{
            border-bottom: 1px solid #eee;
            padding-bottom: 8px;
        }}
        code {{
            background: #f4f4f4;
            padding: 2px 6px;
            border-radius: 3px;
            font-family: "Fira Code", "Consolas", monospace;
        }}
        pre {{
            background: #f4f4f4;
            padding: 16px;
            border-radius: 6px;
            overflow-x: auto;
        }}
        pre code {{
            background: none;
            padding: 0;
        }}
        blockquote {{
            border-left: 4px solid #ddd;
            margin-left: 0;
            padding-left: 16px;
            color: #666;
        }}
        table {{
            border-collapse: collapse;
            width: 100%;
            margin: 16px 0;
        }}
        th, td {{
            border: 1px solid #ddd;
            padding: 8px 12px;
            text-align: left;
        }}
        th {{
            background: #f4f4f4;
        }}
        a {{
            color: #0366d6;
        }}
        img {{
            max-width: 100%;
        }}
        @media print {{
            body {{
                max-width: none;
                padding: 20px;
            }}
            a {{
                color: #333;
                text-decoration: underline;
            }}
        }}
    </style>
</head>
<body>
    <h1>{title}</h1>
    {body_html}
</body>
</html>"#,
        title = html_escape(title),
        body_html = body_html
    )
}

/// Escape special characters for HTML attribute/text contexts.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_format_extension() {
        assert_eq!(ExportFormat::PlainText.extension(), "txt");
        assert_eq!(ExportFormat::Html.extension(), "html");
        assert_eq!(ExportFormat::Markdown.extension(), "md");
    }

    #[test]
    fn test_export_to_html_contains_title() {
        let html = export_to_html("My Note", "Hello world");
        assert!(html.contains("<title>My Note</title>"));
        assert!(html.contains("<h1>My Note</h1>"));
    }

    #[test]
    fn test_export_to_html_contains_content() {
        let html = export_to_html("Test", "**bold** and *italic*");
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<em>italic</em>"));
    }

    #[test]
    fn test_export_to_html_has_print_css() {
        let html = export_to_html("Test", "content");
        assert!(html.contains("@media print"));
    }

    #[test]
    fn test_export_to_html_is_self_contained() {
        let html = export_to_html("Test", "content");
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<html"));
        assert!(html.contains("</html>"));
        assert!(html.contains("<style>"));
    }

    #[test]
    fn test_export_to_html_escapes_title() {
        let html = export_to_html("Test <script>", "content");
        assert!(html.contains("Test &lt;script&gt;"));
        assert!(!html.contains("<title>Test <script>"));
    }

    #[test]
    fn test_export_to_html_tables() {
        let md = "| A | B |\n| --- | --- |\n| 1 | 2 |";
        let html = export_to_html("Test", md);
        assert!(html.contains("<table>"));
        assert!(html.contains("<th>"));
    }

    #[test]
    fn test_export_note_plain_text() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("test.txt");
        export_note("Title", "Hello world", ExportFormat::PlainText, &dest).unwrap();
        let content = std::fs::read_to_string(&dest).unwrap();
        assert_eq!(content, "Hello world");
    }

    #[test]
    fn test_export_note_markdown() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("test.md");
        export_note(
            "Title",
            "# Hello\n\n**bold**",
            ExportFormat::Markdown,
            &dest,
        )
        .unwrap();
        let content = std::fs::read_to_string(&dest).unwrap();
        assert_eq!(content, "# Hello\n\n**bold**");
    }

    #[test]
    fn test_export_note_html() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("test.html");
        export_note("My Note", "Hello", ExportFormat::Html, &dest).unwrap();
        let content = std::fs::read_to_string(&dest).unwrap();
        assert!(content.contains("<!DOCTYPE html>"));
        assert!(content.contains("My Note"));
    }
}
