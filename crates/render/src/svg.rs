use std::fmt::Write as _;

use drawcraft_document::Rect;
use drawcraft_geometry::Vec2;

use crate::scene::{Scene, SceneItem, TextItem};
use crate::text::escape_xml;

/// SVG export options.
#[derive(Clone, Debug, PartialEq)]
pub struct SvgOptions {
    /// Extra world-space margin around the content.
    pub padding: f64,
    /// `None` produces a transparent background.
    pub background: Option<String>,
}

impl Default for SvgOptions {
    fn default() -> Self {
        Self {
            padding: 24.0,
            background: Some("#ffffff".into()),
        }
    }
}

/// Render a scene to a standalone SVG document.
#[must_use]
pub fn scene_to_svg(scene: &Scene, options: &SvgOptions) -> String {
    let mut bounds = scene.bounds;
    if bounds.is_empty() {
        bounds = Rect::from_xywh(0.0, 0.0, 800.0, 600.0);
    }
    let bounds = bounds.expand(options.padding.max(0.0));
    let width = bounds.width().max(1.0);
    let height = bounds.height().max(1.0);

    let mut output = String::with_capacity(2048 + scene.items.len() * 256);
    let _ = write!(
        output,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width:.3}" height="{height:.3}" viewBox="{:.3} {:.3} {width:.3} {height:.3}">"#,
        bounds.min.x, bounds.min.y
    );

    write_defs(&mut output, scene);

    if let Some(background) = &options.background {
        let _ = write!(
            output,
            r##"<rect x="{:.3}" y="{:.3}" width="{width:.3}" height="{height:.3}" fill="{}"/>"##,
            bounds.min.x,
            bounds.min.y,
            escape_xml(background)
        );
    }

    for (index, item) in scene.items.iter().enumerate() {
        write_item(&mut output, item, index);
    }

    output.push_str("</svg>");
    output
}

fn write_defs(output: &mut String, scene: &Scene) {
    let has_defs = scene
        .items
        .iter()
        .any(|item| item.clip.is_some() || item.fills.iter().any(|fill| fill.pattern));
    if !has_defs {
        return;
    }
    output.push_str("<defs>");
    for (index, item) in scene.items.iter().enumerate() {
        if let Some(clip) = item.clip {
            let _ = write!(
                output,
                r#"<clipPath id="clip-{index}"><rect x="{:.3}" y="{:.3}" width="{:.3}" height="{:.3}"/></clipPath>"#,
                clip.min.x,
                clip.min.y,
                clip.width(),
                clip.height()
            );
        }
        for (fill_index, fill) in item.fills.iter().enumerate() {
            if fill.pattern {
                let _ = write!(
                    output,
                    r##"<pattern id="pattern-{index}-{fill_index}" patternUnits="userSpaceOnUse" width="8" height="8"><path d="M0 8 L8 0" stroke="{}" stroke-width="1" stroke-opacity="{:.3}"/></pattern>"##,
                    escape_xml(&fill.color),
                    fill.opacity.clamp(0.0, 1.0)
                );
            }
        }
    }
    output.push_str("</defs>");
}

fn write_item(output: &mut String, item: &SceneItem, index: usize) {
    let _ = write!(
        output,
        r#"<g opacity="{:.3}""#,
        item.opacity.clamp(0.0, 1.0)
    );
    if item.clip.is_some() {
        let _ = write!(output, r#" clip-path="url(#clip-{index})""#);
    }
    output.push('>');

    for (fill_index, fill) in item.fills.iter().enumerate() {
        let fill_value = if fill.pattern {
            format!(r#"url(#pattern-{index}-{fill_index})"#)
        } else {
            escape_xml(&fill.color)
        };
        let _ = write!(
            output,
            r#"<path d="{}" fill="{}" fill-opacity="{:.3}" fill-rule="nonzero"/>"#,
            fill.path.to_svg_d(),
            fill_value,
            fill.opacity.clamp(0.0, 1.0)
        );
    }

    for stroke in &item.strokes {
        let dash = stroke
            .dash
            .dash_array(stroke.width)
            .map(|value| format!(r#" stroke-dasharray="{value}""#))
            .unwrap_or_default();
        let _ = write!(
            output,
            r#"<path d="{}" fill="none" stroke="{}" stroke-opacity="{:.3}" stroke-width="{:.3}" stroke-linecap="round" stroke-linejoin="round"{dash}/>"#,
            stroke.path.to_svg_d(),
            escape_xml(&stroke.color),
            stroke.opacity.clamp(0.0, 1.0),
            stroke.width
        );
    }

    if let Some(text) = &item.text {
        write_text(output, text);
    }

    if let Some(image) = &item.image {
        let _ = write!(
            output,
            r#"<image href="{}" x="{:.3}" y="{:.3}" width="{:.3}" height="{:.3}" preserveAspectRatio="xMidYMid meet"/>"#,
            escape_xml(&image.href),
            image.rect.min.x,
            image.rect.min.y,
            image.rect.width(),
            image.rect.height()
        );
    }

    output.push_str("</g>");
}

fn write_text(output: &mut String, text: &TextItem) {
    let lines = text.lines();
    if lines.iter().all(String::is_empty) {
        return;
    }
    let rotated = text.rotation.abs() > f64::EPSILON;
    if rotated {
        let _ = write!(
            output,
            r#"<g transform="rotate({:.3} {:.3} {:.3})">"#,
            text.rotation.to_degrees(),
            text.rotation_center.x,
            text.rotation_center.y
        );
    }
    let x = text.anchor_x();
    let line_height = crate::text::line_height(text.font_size);
    // `sans` is not a generic CSS family; map it to the portable generic name.
    let font_family = match text.font_family.as_str() {
        "" | "sans" => "sans-serif",
        other => other,
    };
    for (index, line) in lines.iter().enumerate() {
        let y = text.pos.y + text.font_size + index as f64 * line_height;
        let _ = write!(
            output,
            r#"<text x="{x:.3}" y="{y:.3}" font-size="{:.3}" font-family="{}" fill="{}" fill-opacity="{:.3}" text-anchor="{}" xml:space="preserve">{}</text>"#,
            text.font_size,
            escape_xml(font_family),
            escape_xml(&text.color),
            text.opacity.clamp(0.0, 1.0),
            text.align.as_svg(),
            escape_xml(line)
        );
    }
    if rotated {
        output.push_str("</g>");
    }
}

/// Convenience: render a single path as a minimal SVG for tests and previews.
#[must_use]
pub fn path_to_svg(path: &crate::path::Path, stroke: &str, width: f64) -> String {
    let mut output = String::new();
    let _ = write!(
        output,
        r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="{}" fill="none" stroke="{}" stroke-width="{width:.3}" stroke-linecap="round"/></svg>"#,
        path.to_svg_d(),
        escape_xml(stroke)
    );
    output
}

/// Default view box for an empty scene.
#[must_use]
pub fn empty_viewport() -> Rect {
    Rect::new(Vec2::ZERO, Vec2::new(800.0, 600.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::SceneItem;
    use crate::shapes::shape_path;
    use drawcraft_document::{Dash, ShapeKind};

    fn scene_with_shape() -> Scene {
        let shape = ShapeKind::Rectangle;
        let item = SceneItem {
            id: None,
            kind: shape,
            z: 0,
            fills: vec![crate::scene::FillPaint {
                path: shape_path(shape, Rect::from_xywh(10.0, 20.0, 100.0, 60.0)),
                color: "#74c7ec".into(),
                opacity: 0.5,
                pattern: false,
            }],
            strokes: vec![crate::scene::StrokePaint {
                path: shape_path(shape, Rect::from_xywh(10.0, 20.0, 100.0, 60.0)),
                color: "#1e1e2e".into(),
                width: 2.0,
                dash: Dash::Dashed,
                opacity: 1.0,
            }],
            hatch: Vec::new(),
            text: None,
            image: None,
            rotation: 0.0,
            rotation_center: Vec2::ZERO,
            opacity: 1.0,
            locked: false,
            clip: None,
            group: None,
        };
        Scene {
            items: vec![item],
            bounds: Rect::from_xywh(10.0, 20.0, 100.0, 60.0),
        }
    }

    #[test]
    fn svg_contains_geometry_and_dash() {
        let svg = scene_to_svg(&scene_with_shape(), &SvgOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("stroke-dasharray"));
        assert!(svg.contains("<path"));
    }

    #[test]
    fn transparent_background_omits_rect() {
        let options = SvgOptions {
            background: None,
            ..SvgOptions::default()
        };
        let svg = scene_to_svg(&scene_with_shape(), &options);
        assert!(!svg.contains(r##"fill="#ffffff""##));
    }
}
