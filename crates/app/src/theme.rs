//! Visual system for DrawCraft.
//!
//! The palette follows the GrokDay/GrokNight design language: a neutral gray
//! base, a single magenta assistant accent, and a small set of semantic hues.
//! Light is the default DrawCraft theme; dark is the Night variant.

use drawcraft_document::{Color, parse_color};
use egui::{Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle};

/// GrokDay/GrokNight role palette.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Palette {
    pub main: Color32,
    pub terminal: Color32,
    pub surface: Color32,
    pub highlight: Color32,
    pub code: Color32,
    pub text: Color32,
    pub secondary: Color32,
    pub muted: Color32,
    pub accent: Color32,
    pub system: Color32,
    pub error: Color32,
    pub success: Color32,
    pub plan: Color32,
    pub heading: Color32,
    pub warning: Color32,
    pub path: Color32,
    pub running: Color32,
    pub inline_code: Color32,
    pub remember: Color32,
    pub diff_delete: Color32,
    pub diff_insert: Color32,
    /// Text drawn on top of `accent`.
    pub on_accent: Color32,
}

/// GrokDay: light theme.
pub const DAY: Palette = Palette {
    main: Color32::from_rgb(0xee, 0xee, 0xee),
    terminal: Color32::from_rgb(0xf5, 0xf5, 0xf5),
    surface: Color32::from_rgb(0xea, 0xea, 0xea),
    highlight: Color32::from_rgb(0xde, 0xde, 0xde),
    code: Color32::from_rgb(0xe4, 0xe4, 0xe4),
    text: Color32::from_rgb(0x26, 0x26, 0x26),
    secondary: Color32::from_rgb(0x44, 0x44, 0x44),
    muted: Color32::from_rgb(0x76, 0x76, 0x76),
    accent: Color32::from_rgb(0x7d, 0x4b, 0xc6),
    system: Color32::from_rgb(0x2f, 0x64, 0xd2),
    error: Color32::from_rgb(0xcd, 0x30, 0x48),
    success: Color32::from_rgb(0x37, 0x8e, 0x23),
    plan: Color32::from_rgb(0xa8, 0x78, 0x0a),
    heading: Color32::from_rgb(0x0a, 0x8e, 0x70),
    warning: Color32::from_rgb(0xa2, 0x76, 0x12),
    path: Color32::from_rgb(0xc3, 0x69, 0x1e),
    running: Color32::from_rgb(0x00, 0x82, 0xaa),
    inline_code: Color32::from_rgb(0x0f, 0x87, 0xa2),
    remember: Color32::from_rgb(0x4c, 0xaf, 0x50),
    diff_delete: Color32::from_rgb(0xf5, 0xda, 0xde),
    diff_insert: Color32::from_rgb(0xda, 0xf2, 0xdc),
    on_accent: Color32::WHITE,
};

/// GrokNight: dark theme.
pub const NIGHT: Palette = Palette {
    main: Color32::from_rgb(0x14, 0x14, 0x14),
    terminal: Color32::from_rgb(0x0a, 0x0a, 0x0a),
    surface: Color32::from_rgb(0x11, 0x11, 0x11),
    highlight: Color32::from_rgb(0x24, 0x24, 0x24),
    code: Color32::from_rgb(0x1c, 0x1c, 0x1c),
    text: Color32::from_rgb(0xe1, 0xe1, 0xe1),
    secondary: Color32::from_rgb(0xc8, 0xc8, 0xc8),
    muted: Color32::from_rgb(0x6c, 0x6c, 0x6c),
    accent: Color32::from_rgb(0xbb, 0x9a, 0xf7),
    system: Color32::from_rgb(0x7a, 0xa2, 0xf7),
    error: Color32::from_rgb(0xf7, 0x76, 0x8e),
    success: Color32::from_rgb(0x9e, 0xce, 0x6a),
    plan: Color32::from_rgb(0xff, 0xdb, 0x8d),
    heading: Color32::from_rgb(0x1a, 0xbc, 0x9c),
    warning: Color32::from_rgb(0xe0, 0xaf, 0x68),
    path: Color32::from_rgb(0xff, 0x9e, 0x64),
    running: Color32::from_rgb(0x7d, 0xcf, 0xff),
    inline_code: Color32::from_rgb(0x3a, 0x95, 0xab),
    remember: Color32::from_rgb(0x8b, 0xc3, 0x4a),
    diff_delete: Color32::from_rgb(0x42, 0x0e, 0x14),
    diff_insert: Color32::from_rgb(0x06, 0x38, 0x06),
    on_accent: Color32::BLACK,
};

/// Return the palette for the requested theme.
#[must_use]
pub const fn palette(dark: bool) -> Palette {
    if dark { NIGHT } else { DAY }
}

/// Resolved canvas colours.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CanvasColors {
    pub background: Color32,
    pub surface: Color32,
    pub grid: Color32,
    pub grid_major: Color32,
    pub text: Color32,
    pub secondary: Color32,
    pub muted: Color32,
    pub selection: Color32,
    pub handle_fill: Color32,
    pub handle_stroke: Color32,
    pub marquee_fill: Color32,
    pub accent: Color32,
    pub success: Color32,
    pub error: Color32,
    pub border: Color32,
}

/// Canvas colors derived from the active palette.
#[must_use]
pub fn canvas_colors(dark: bool) -> CanvasColors {
    let palette = palette(dark);
    CanvasColors {
        background: palette.terminal,
        surface: palette.surface,
        grid: if dark {
            palette.code
        } else {
            Color32::from_rgb(0xe4, 0xe4, 0xe4)
        },
        grid_major: palette.highlight,
        text: palette.text,
        secondary: palette.secondary,
        muted: palette.muted,
        selection: palette.accent,
        handle_fill: if dark { palette.main } else { Color32::WHITE },
        handle_stroke: palette.accent,
        marquee_fill: with_alpha(palette.accent, 30),
        accent: palette.accent,
        success: palette.success,
        error: palette.error,
        border: palette.highlight,
    }
}

/// Apply the GrokDay/GrokNight widget theme.
pub fn apply(ctx: &egui::Context, dark: bool) {
    let palette = palette(dark);
    let mut visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    visuals.dark_mode = dark;
    visuals.panel_fill = palette.surface;
    visuals.window_fill = palette.surface;
    visuals.extreme_bg_color = palette.code;
    visuals.text_edit_bg_color = Some(palette.code);
    visuals.code_bg_color = palette.code;
    visuals.faint_bg_color = palette.highlight;
    visuals.override_text_color = Some(palette.text);
    visuals.weak_text_color = Some(palette.muted);
    visuals.hyperlink_color = palette.system;
    visuals.warn_fg_color = palette.warning;
    visuals.error_fg_color = palette.error;
    visuals.window_stroke = Stroke::new(1.0, palette.highlight);
    visuals.window_corner_radius = CornerRadius::same(12);
    visuals.menu_corner_radius = CornerRadius::same(10);
    visuals.window_highlight_topmost = false;
    visuals.button_frame = true;
    visuals.striped = true;
    visuals.slider_trailing_fill = true;

    visuals.selection.bg_fill = with_alpha(palette.accent, if dark { 48 } else { 40 });
    visuals.selection.stroke = Stroke::new(1.0, palette.accent);
    visuals.widgets.noninteractive.bg_fill = palette.surface;
    visuals.widgets.noninteractive.weak_bg_fill = palette.surface;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, palette.highlight);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, palette.text);
    visuals.widgets.inactive.bg_fill = palette.surface;
    visuals.widgets.inactive.weak_bg_fill = palette.surface;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, palette.highlight);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, palette.secondary);
    visuals.widgets.hovered.bg_fill = palette.highlight;
    visuals.widgets.hovered.weak_bg_fill = palette.highlight;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, palette.muted);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, palette.text);
    visuals.widgets.active.bg_fill = palette.accent;
    visuals.widgets.active.weak_bg_fill = palette.accent;
    visuals.widgets.active.bg_stroke = Stroke::NONE;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, palette.on_accent);
    visuals.widgets.open.bg_fill = palette.highlight;
    visuals.widgets.open.weak_bg_fill = palette.highlight;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, palette.highlight);
    visuals.widgets.open.fg_stroke = Stroke::new(1.0, palette.text);

    ctx.set_theme(if dark {
        egui::ThemePreference::Dark
    } else {
        egui::ThemePreference::Light
    });
    ctx.set_visuals(visuals);
    let theme = ctx.theme();
    ctx.style_mut_of(theme, |style| {
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 5.0);
        style.spacing.interact_size.y = 28.0;
        style.spacing.scroll.bar_width = 8.0;
        style.spacing.scroll.bar_inner_margin = 4.0;
        style.spacing.scroll.bar_outer_margin = 2.0;
        style.spacing.slider_width = 132.0;
        style.spacing.combo_width = 132.0;
        style.spacing.scroll.floating = true;
        style.spacing.indent = 18.0;
        style.text_styles = [
            (
                TextStyle::Heading,
                FontId::new(20.0, FontFamily::Proportional),
            ),
            (TextStyle::Body, FontId::new(13.0, FontFamily::Proportional)),
            (
                TextStyle::Button,
                FontId::new(12.5, FontFamily::Proportional),
            ),
            (
                TextStyle::Small,
                FontId::new(10.5, FontFamily::Proportional),
            ),
            (
                TextStyle::Monospace,
                FontId::new(12.0, FontFamily::Monospace),
            ),
        ]
        .into();
    });
}

/// A palette-aware subtle surface for cards and inspector sections.
#[must_use]
pub fn card_fill(dark: bool) -> Color32 {
    palette(dark).main
}

#[must_use]
pub fn card_stroke(dark: bool) -> Stroke {
    Stroke::new(1.0, palette(dark).highlight)
}

#[must_use]
pub fn with_alpha(color: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}

/// Parse a document color and apply an opacity multiplier.
#[must_use]
pub fn color(value: &str, opacity: f64) -> Color32 {
    let color = parse_color(value).unwrap_or(Color::BLACK);
    let alpha = (f64::from(color.a) * opacity.clamp(0.0, 1.0)).round();
    Color32::from_rgba_unmultiplied(color.r, color.g, color.b, alpha as u8)
}

/// Solid color without opacity scaling.
#[must_use]
pub fn solid(value: &str) -> Color32 {
    color(value, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_matches_grok_palette() {
        assert_eq!(DAY.main, Color32::from_rgb(0xee, 0xee, 0xee));
        assert_eq!(DAY.accent, Color32::from_rgb(0x7d, 0x4b, 0xc6));
        assert_eq!(DAY.text, Color32::from_rgb(0x26, 0x26, 0x26));
        assert_eq!(DAY.terminal, Color32::from_rgb(0xf5, 0xf5, 0xf5));
    }

    #[test]
    fn night_matches_grok_palette() {
        assert_eq!(NIGHT.main, Color32::from_rgb(0x14, 0x14, 0x14));
        assert_eq!(NIGHT.accent, Color32::from_rgb(0xbb, 0x9a, 0xf7));
        assert_eq!(NIGHT.text, Color32::from_rgb(0xe1, 0xe1, 0xe1));
    }

    #[test]
    fn canvas_uses_terminal_surface() {
        assert_eq!(canvas_colors(false).background, DAY.terminal);
        assert_eq!(canvas_colors(true).background, NIGHT.terminal);
    }
}
