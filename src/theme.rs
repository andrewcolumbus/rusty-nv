use crate::highlight::HighlightColors;
use catppuccin::FlavorColors;
use egui::{style::WidgetVisuals, Color32, CornerRadius, Stroke, Style, Visuals};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeChoice {
    Latte,
    Frappe,
    Macchiato,
    #[default]
    Mocha,
}

impl ThemeChoice {
    pub fn apply(&self, ctx: &egui::Context) {
        let flavor = self.flavor();
        let visuals = make_visuals(&flavor, self.is_dark());
        ctx.set_style(Style {
            visuals,
            ..Style::default()
        });
    }

    fn flavor(&self) -> FlavorColors {
        match self {
            ThemeChoice::Latte => catppuccin::PALETTE.latte.colors,
            ThemeChoice::Frappe => catppuccin::PALETTE.frappe.colors,
            ThemeChoice::Macchiato => catppuccin::PALETTE.macchiato.colors,
            ThemeChoice::Mocha => catppuccin::PALETTE.mocha.colors,
        }
    }

    fn is_dark(&self) -> bool {
        !matches!(self, ThemeChoice::Latte)
    }

    pub fn label(&self) -> &'static str {
        match self {
            ThemeChoice::Latte => "Latte (Light)",
            ThemeChoice::Frappe => "Frappe",
            ThemeChoice::Macchiato => "Macchiato",
            ThemeChoice::Mocha => "Mocha (Dark)",
        }
    }

    pub fn toggle(&self) -> Self {
        match self {
            ThemeChoice::Latte => ThemeChoice::Mocha,
            ThemeChoice::Mocha => ThemeChoice::Latte,
            ThemeChoice::Frappe => ThemeChoice::Macchiato,
            ThemeChoice::Macchiato => ThemeChoice::Frappe,
        }
    }

    #[allow(dead_code)]
    pub fn all() -> &'static [ThemeChoice] {
        &[
            ThemeChoice::Latte,
            ThemeChoice::Frappe,
            ThemeChoice::Macchiato,
            ThemeChoice::Mocha,
        ]
    }
}

fn ctp_color(c: catppuccin::Color) -> Color32 {
    Color32::from_rgb(c.rgb.r, c.rgb.g, c.rgb.b)
}

/// Return highlight colors appropriate for the given theme.
///
/// Uses catppuccin palette colors:
/// - yellow at 40% alpha for search background
/// - peach at 60% alpha for active search background
/// - sapphire for wiki link color
pub fn highlight_colors(theme: ThemeChoice) -> HighlightColors {
    let f = theme.flavor();
    let text = ctp_color(f.text);
    let yellow = ctp_color(f.yellow);
    let peach = ctp_color(f.peach);
    let sapphire = ctp_color(f.sapphire);

    HighlightColors {
        default_text: text,
        search_bg: Color32::from_rgba_unmultiplied(yellow.r(), yellow.g(), yellow.b(), 102), // ~40%
        active_search_bg: Color32::from_rgba_unmultiplied(peach.r(), peach.g(), peach.b(), 153), // ~60%
        link_color: sapphire,
    }
}

fn make_visuals(f: &FlavorColors, dark: bool) -> Visuals {
    let base = ctp_color(f.base);
    let mantle = ctp_color(f.mantle);
    let crust = ctp_color(f.crust);
    let surface0 = ctp_color(f.surface0);
    let surface1 = ctp_color(f.surface1);
    let surface2 = ctp_color(f.surface2);
    let text = ctp_color(f.text);
    let blue = ctp_color(f.blue);
    let lavender = ctp_color(f.lavender);
    let red = ctp_color(f.red);

    let corner_radius = CornerRadius::same(5);

    let widget_inactive = WidgetVisuals {
        bg_fill: surface0,
        weak_bg_fill: surface0,
        bg_stroke: Stroke::new(1.0, surface1),
        corner_radius,
        fg_stroke: Stroke::new(1.0, text),
        expansion: 0.0,
    };

    let widget_hovered = WidgetVisuals {
        bg_fill: surface1,
        weak_bg_fill: surface1,
        bg_stroke: Stroke::new(1.0, blue),
        corner_radius,
        fg_stroke: Stroke::new(1.0, text),
        expansion: 1.0,
    };

    let widget_active = WidgetVisuals {
        bg_fill: surface2,
        weak_bg_fill: surface2,
        bg_stroke: Stroke::new(1.0, lavender),
        corner_radius,
        fg_stroke: Stroke::new(1.0, text),
        expansion: 1.0,
    };

    let widget_open = WidgetVisuals {
        bg_fill: surface1,
        weak_bg_fill: surface1,
        bg_stroke: Stroke::new(1.0, blue),
        corner_radius,
        fg_stroke: Stroke::new(1.0, text),
        expansion: 0.0,
    };

    let widget_noninteractive = WidgetVisuals {
        bg_fill: base,
        weak_bg_fill: mantle,
        bg_stroke: Stroke::new(1.0, surface0),
        corner_radius,
        fg_stroke: Stroke::new(1.0, text),
        expansion: 0.0,
    };

    let shadow_alpha = if dark { 60 } else { 20 };

    Visuals {
        dark_mode: dark,
        override_text_color: Some(text),
        hyperlink_color: blue,
        faint_bg_color: mantle,
        extreme_bg_color: crust,
        code_bg_color: mantle,
        warn_fg_color: ctp_color(f.peach),
        error_fg_color: red,
        window_fill: base,
        window_stroke: Stroke::new(1.0, surface0),
        window_shadow: egui::Shadow {
            spread: 0,
            blur: 8,
            offset: [0, 2],
            color: Color32::from_black_alpha(shadow_alpha),
        },
        window_corner_radius: CornerRadius::same(8),
        panel_fill: base,
        popup_shadow: egui::Shadow {
            spread: 0,
            blur: 6,
            offset: [0, 2],
            color: Color32::from_black_alpha(shadow_alpha / 2),
        },
        selection: egui::style::Selection {
            bg_fill: blue.linear_multiply(0.3),
            stroke: Stroke::new(1.0, blue),
        },
        widgets: egui::style::Widgets {
            noninteractive: widget_noninteractive,
            inactive: widget_inactive,
            hovered: widget_hovered,
            active: widget_active,
            open: widget_open,
        },
        text_cursor: egui::style::TextCursorStyle {
            stroke: Stroke::new(2.0, lavender),
            ..Default::default()
        },
        striped: false,
        slider_trailing_fill: true,
        handle_shape: egui::style::HandleShape::Circle,
        interact_cursor: None,
        image_loading_spinners: true,
        numeric_color_space: egui::style::NumericColorSpace::GammaByte,
        indent_has_left_vline: true,
        resize_corner_size: 12.0,
        clip_rect_margin: 3.0,
        button_frame: true,
        collapsing_header_frame: false,
        ..Visuals::default()
    }
}
