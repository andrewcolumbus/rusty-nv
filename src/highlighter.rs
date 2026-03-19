//! Builds an egui `LayoutJob` from markup parser output.
//!
//! NOTE: This module's functionality has been integrated into
//! `highlight::build_combined_layout_job()` which merges markdown formatting
//! with search/wiki highlighting in a single pass. This module is retained
//! as a reference and for its tests.
#![allow(dead_code)]

use crate::markup;
use egui::text::{LayoutJob, LayoutSection};
use egui::{Color32, FontFamily, FontId, Stroke, TextFormat};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;

/// Cached layout job to avoid re-computing every frame.
struct CachedJob {
    text_hash: u64,
    style_hash: u64,
    font_size: u32, // f32 bits
    job: LayoutJob,
}

static CACHE: Mutex<Option<CachedJob>> = Mutex::new(None);

/// Build a `LayoutJob` from markdown-formatted text.
///
/// Uses the current egui style for color information and applies markdown
/// formatting as `TextFormat` sections.
///
/// Results are memoized: if `text`, `ui_style`, and `font_size` haven't changed
/// since the last call, the cached `LayoutJob` is returned.
pub fn highlight(ui_style: &egui::Style, text: &str, font_size: f32) -> LayoutJob {
    let text_hash = hash_str(text);
    let style_hash = hash_style(ui_style);
    let font_size_bits = font_size.to_bits();

    // Check cache
    if let Ok(guard) = CACHE.lock() {
        if let Some(ref cached) = *guard {
            if cached.text_hash == text_hash
                && cached.style_hash == style_hash
                && cached.font_size == font_size_bits
            {
                return cached.job.clone();
            }
        }
    }

    // Build fresh job
    let job = build_layout_job(ui_style, text, font_size);

    // Update cache
    if let Ok(mut guard) = CACHE.lock() {
        *guard = Some(CachedJob {
            text_hash,
            style_hash,
            font_size: font_size_bits,
            job: job.clone(),
        });
    }

    job
}

/// Build the layout job from scratch (no caching).
fn build_layout_job(ui_style: &egui::Style, text: &str, font_size: f32) -> LayoutJob {
    let spans = markup::parse(text);

    let text_color = ui_style
        .visuals
        .override_text_color
        .unwrap_or(Color32::GRAY);
    let code_bg = ui_style.visuals.code_bg_color;
    let link_color = ui_style.visuals.hyperlink_color;
    let dimmed_color = if ui_style.visuals.dark_mode {
        Color32::from_rgba_premultiplied(160, 160, 175, 255)
    } else {
        Color32::from_rgba_premultiplied(100, 100, 110, 255)
    };

    let default_font = FontId::new(font_size, FontFamily::Proportional);
    let monospace_font = FontId::new(font_size, FontFamily::Monospace);

    let mut job = LayoutJob {
        text: text.to_owned(),
        ..Default::default()
    };

    for (range, style) in spans {
        let format = style_to_format(
            &style,
            &default_font,
            &monospace_font,
            font_size,
            text_color,
            code_bg,
            link_color,
            dimmed_color,
        );

        job.sections.push(LayoutSection {
            leading_space: 0.0,
            byte_range: range,
            format,
        });
    }

    // If no sections were produced (shouldn't happen with the parser, but defensive),
    // add a single default section.
    if job.sections.is_empty() && !text.is_empty() {
        job.sections.push(LayoutSection {
            leading_space: 0.0,
            byte_range: 0..text.len(),
            format: TextFormat {
                font_id: default_font,
                color: text_color,
                ..Default::default()
            },
        });
    }

    job
}

/// Convert a markup `Style` to an egui `TextFormat`.
#[allow(clippy::too_many_arguments)]
fn style_to_format(
    style: &markup::Style,
    default_font: &FontId,
    monospace_font: &FontId,
    base_size: f32,
    text_color: Color32,
    code_bg: Color32,
    link_color: Color32,
    dimmed_color: Color32,
) -> TextFormat {
    let mut format = TextFormat {
        font_id: default_font.clone(),
        color: text_color,
        ..Default::default()
    };

    // Heading: proportionally larger font
    if style.heading > 0 {
        let scale = match style.heading {
            1 => 1.6,
            2 => 1.4,
            3 => 1.25,
            4 => 1.15,
            5 => 1.1,
            6 => 1.05,
            _ => 1.0,
        };
        format.font_id = FontId::new(base_size * scale, FontFamily::Proportional);
    }

    // Bold: use stronger color + slightly larger size (no bold font guaranteed)
    if style.bold {
        // Brighten the text color slightly for emphasis
        format.color = brighten_color(text_color, 20);
        let current_size = format.font_id.size;
        format.font_id = FontId::new(current_size * 1.02, format.font_id.family.clone());
    }

    // Italic
    if style.italic {
        format.italics = true;
    }

    // Strikethrough
    if style.strikethrough {
        format.strikethrough = Stroke::new(1.0, text_color);
    }

    // Code: monospace + subtle background
    if style.code {
        format.font_id = monospace_font.clone();
        format.background = code_bg;
    }

    // Quote: dimmed color + italic
    if style.quote {
        format.color = dimmed_color;
        format.italics = true;
    }

    // URL: link color + underline
    if style.url {
        format.color = link_color;
        format.underline = Stroke::new(1.0, link_color);
    }

    format
}

/// Brighten a color by adding `amount` to each RGB channel (clamped to 255).
fn brighten_color(color: Color32, amount: u8) -> Color32 {
    Color32::from_rgba_premultiplied(
        color.r().saturating_add(amount),
        color.g().saturating_add(amount),
        color.b().saturating_add(amount),
        color.a(),
    )
}

/// Hash a string for cache comparison.
fn hash_str(s: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

/// Hash relevant parts of an egui Style for cache comparison.
fn hash_style(style: &egui::Style) -> u64 {
    let mut hasher = DefaultHasher::new();
    // Hash a few key visual properties
    if let Some(c) = style.visuals.override_text_color {
        c.to_array().hash(&mut hasher);
    }
    style.visuals.code_bg_color.to_array().hash(&mut hasher);
    style.visuals.hyperlink_color.to_array().hash(&mut hasher);
    style.visuals.dark_mode.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_style() -> egui::Style {
        egui::Style::default()
    }

    #[test]
    fn test_highlight_empty_text() {
        let style = default_style();
        let job = highlight(&style, "", 14.0);
        assert!(job.sections.is_empty());
        assert!(job.text.is_empty());
    }

    #[test]
    fn test_highlight_plain_text() {
        let style = default_style();
        let job = highlight(&style, "Hello, world!", 14.0);
        assert!(!job.sections.is_empty());
        assert_eq!(job.text, "Hello, world!");

        // All sections should cover the full text
        let total_bytes: usize = job.sections.iter().map(|s| s.byte_range.len()).sum();
        assert_eq!(total_bytes, "Hello, world!".len());
    }

    #[test]
    fn test_highlight_with_bold() {
        let style = default_style();
        let job = highlight(&style, "before **bold** after", 14.0);

        // Should have multiple sections
        assert!(job.sections.len() > 1);

        // Total byte coverage should equal text length
        let total_bytes: usize = job.sections.iter().map(|s| s.byte_range.len()).sum();
        assert_eq!(total_bytes, "before **bold** after".len());
    }

    #[test]
    fn test_highlight_with_code() {
        let style = default_style();
        let job = highlight(&style, "before `code` after", 14.0);

        // Find the code section (should have monospace font)
        let code_section = job
            .sections
            .iter()
            .find(|s| s.format.font_id.family == FontFamily::Monospace);
        assert!(code_section.is_some());
    }

    #[test]
    fn test_highlight_with_url() {
        let style = default_style();
        let text = "Visit https://example.com today";
        let job = highlight(&style, text, 14.0);

        // Find the URL section (should have underline)
        let url_section = job
            .sections
            .iter()
            .find(|s| s.format.underline != Stroke::NONE);
        assert!(url_section.is_some());
    }

    #[test]
    fn test_highlight_heading_larger_font() {
        let style = default_style();
        let base_size = 14.0;
        let job = highlight(&style, "# Big Heading", base_size);

        // All sections should have larger font size
        for section in &job.sections {
            assert!(
                section.format.font_id.size > base_size,
                "Heading font size {} should be > base {}",
                section.format.font_id.size,
                base_size
            );
        }
    }

    #[test]
    fn test_highlight_italic() {
        let style = default_style();
        let job = highlight(&style, "before *italic* after", 14.0);

        let italic_section = job.sections.iter().find(|s| s.format.italics);
        assert!(italic_section.is_some());
    }

    #[test]
    fn test_highlight_strikethrough() {
        let style = default_style();
        let job = highlight(&style, "before ~~struck~~ after", 14.0);

        let strike_section = job
            .sections
            .iter()
            .find(|s| s.format.strikethrough != Stroke::NONE);
        assert!(strike_section.is_some());
    }

    #[test]
    fn test_highlight_caching() {
        let style = default_style();
        let text = "Cached **text** test";

        let job1 = highlight(&style, text, 14.0);
        let job2 = highlight(&style, text, 14.0);

        // Both should return the same number of sections (cached)
        assert_eq!(job1.sections.len(), job2.sections.len());
    }

    #[test]
    fn test_highlight_cache_invalidation() {
        let style = default_style();

        let job1 = highlight(&style, "**text1**", 14.0);
        let job2 = highlight(&style, "**text2**", 14.0);

        // Different text should produce different jobs
        assert_eq!(job1.text, "**text1**");
        assert_eq!(job2.text, "**text2**");
    }

    #[test]
    fn test_highlight_quote_style() {
        let style = default_style();
        let job = highlight(&style, "> Quoted text", 14.0);

        // Quote sections should be italic
        let quote_section = job.sections.iter().find(|s| s.format.italics);
        assert!(quote_section.is_some());
    }

    #[test]
    fn test_brighten_color() {
        let c = Color32::from_rgb(100, 200, 50);
        let bright = brighten_color(c, 30);
        assert_eq!(bright.r(), 130);
        assert_eq!(bright.g(), 230);
        assert_eq!(bright.b(), 80);
    }

    #[test]
    fn test_brighten_color_clamps() {
        let c = Color32::from_rgb(250, 250, 250);
        let bright = brighten_color(c, 30);
        assert_eq!(bright.r(), 255);
        assert_eq!(bright.g(), 255);
        assert_eq!(bright.b(), 255);
    }

    #[test]
    fn test_sections_cover_all_bytes() {
        let style = default_style();
        let texts = vec![
            "# Heading\n**bold** and *italic*\n> quote with `code`",
            "https://example.com **bold**",
            "~~strike~~ _under_",
            "plain text no formatting",
            "## H2\n### H3\n#### H4",
        ];

        for text in texts {
            let job = highlight(&style, text, 14.0);
            let total: usize = job.sections.iter().map(|s| s.byte_range.len()).sum();
            assert_eq!(total, text.len(), "Byte coverage mismatch for: {:?}", text);
        }
    }
}
