use drawcraft_document::{ArrowHead, Rect, ShapeKind, Vec2};

use crate::path::Path;

/// Build the outline for a closed shape kind.
#[must_use]
pub fn shape_path(kind: ShapeKind, rect: Rect) -> Path {
    let w = rect.width();
    let h = rect.height();
    let (x, y) = (rect.min.x, rect.min.y);
    match kind {
        ShapeKind::Rectangle
        | ShapeKind::Text
        | ShapeKind::Sticky
        | ShapeKind::Image
        | ShapeKind::Note
        | ShapeKind::Frame
        | ShapeKind::Group => {
            if kind == ShapeKind::Note {
                Path::note(rect)
            } else {
                Path::rect(rect)
            }
        }
        ShapeKind::Ellipse => Path::ellipse(rect),
        ShapeKind::Triangle => Path::polygon(&[
            Vec2::new(rect.center().x, y),
            Vec2::new(rect.max.x, rect.max.y),
            Vec2::new(x, rect.max.y),
        ]),
        ShapeKind::Diamond => Path::polygon(&[
            Vec2::new(rect.center().x, y),
            Vec2::new(rect.max.x, rect.center().y),
            Vec2::new(rect.center().x, rect.max.y),
            Vec2::new(x, rect.center().y),
        ]),
        ShapeKind::Pentagon => Path::regular_polygon(rect, 5),
        ShapeKind::Hexagon => Path::regular_polygon(rect, 6),
        ShapeKind::Octagon => Path::regular_polygon(rect, 8),
        ShapeKind::Star => Path::star(rect, 5, 0.45),
        ShapeKind::Heart => Path::heart(rect),
        ShapeKind::RoundRect => Path::rounded_rect(rect, (w.min(h) * 0.16).max(2.0)),
        ShapeKind::Parallelogram => {
            let skew = w * 0.2;
            Path::polygon(&[
                Vec2::new(x + skew, y),
                Vec2::new(rect.max.x, y),
                Vec2::new(rect.max.x - skew, rect.max.y),
                Vec2::new(x, rect.max.y),
            ])
        }
        ShapeKind::Trapezoid => {
            let skew = w * 0.2;
            Path::polygon(&[
                Vec2::new(x + skew, y),
                Vec2::new(rect.max.x - skew, y),
                Vec2::new(rect.max.x, rect.max.y),
                Vec2::new(x, rect.max.y),
            ])
        }
        ShapeKind::ArrowShape => Path::polygon(&[
            Vec2::new(x, y + h * 0.25),
            Vec2::new(x + w * 0.62, y + h * 0.25),
            Vec2::new(x + w * 0.62, y),
            Vec2::new(rect.max.x, rect.center().y),
            Vec2::new(x + w * 0.62, rect.max.y),
            Vec2::new(x + w * 0.62, y + h * 0.75),
            Vec2::new(x, y + h * 0.75),
        ]),
        ShapeKind::DoubleArrow => Path::polygon(&[
            Vec2::new(rect.center().x, y),
            Vec2::new(x + w * 0.75, y + h * 0.25),
            Vec2::new(x + w * 0.6, y + h * 0.25),
            Vec2::new(x + w * 0.6, y + h * 0.75),
            Vec2::new(x + w * 0.75, y + h * 0.75),
            Vec2::new(rect.center().x, rect.max.y),
            Vec2::new(x + w * 0.25, y + h * 0.75),
            Vec2::new(x + w * 0.4, y + h * 0.75),
            Vec2::new(x + w * 0.4, y + h * 0.25),
            Vec2::new(x + w * 0.25, y + h * 0.25),
        ]),
        ShapeKind::Cross => Path::polygon(&[
            Vec2::new(x + w * 0.35, y),
            Vec2::new(x + w * 0.65, y),
            Vec2::new(x + w * 0.65, y + h * 0.35),
            Vec2::new(rect.max.x, y + h * 0.35),
            Vec2::new(rect.max.x, y + h * 0.65),
            Vec2::new(x + w * 0.65, y + h * 0.65),
            Vec2::new(x + w * 0.65, rect.max.y),
            Vec2::new(x + w * 0.35, rect.max.y),
            Vec2::new(x + w * 0.35, y + h * 0.65),
            Vec2::new(x, y + h * 0.65),
            Vec2::new(x, y + h * 0.35),
            Vec2::new(x + w * 0.35, y + h * 0.35),
        ]),
        ShapeKind::HomePlate => Path::polygon(&[
            Vec2::new(x, y),
            Vec2::new(x + w * 0.75, y),
            Vec2::new(rect.max.x, rect.center().y),
            Vec2::new(x + w * 0.75, rect.max.y),
            Vec2::new(x, rect.max.y),
        ]),
        ShapeKind::Chevron => Path::polygon(&[
            Vec2::new(x, y),
            Vec2::new(x + w * 0.75, y),
            Vec2::new(rect.max.x, rect.center().y),
            Vec2::new(x + w * 0.75, rect.max.y),
            Vec2::new(x, rect.max.y),
            Vec2::new(x + w * 0.25, rect.center().y),
        ]),
        ShapeKind::Cylinder => Path::cylinder(rect),
        ShapeKind::Cloud => Path::cloud(rect),
        ShapeKind::Draw
        | ShapeKind::Highlighter
        | ShapeKind::Line
        | ShapeKind::Polyline
        | ShapeKind::Arrow
        | ShapeKind::Laser => Path::new(),
    }
}

/// Path for a linear shape. Empty point lists fall back to the rect diagonal.
#[must_use]
pub fn linear_path(points: &[Vec2], fallback: Rect) -> Path {
    if points.len() >= 2 {
        Path::polyline(points)
    } else if let (Some(first), Some(last)) = (points.first(), points.last()) {
        Path::polyline(&[*first, *last])
    } else {
        Path::polyline(&[fallback.min, fallback.max])
    }
}

/// Arrowhead geometry. Filled heads are returned as closed paths; the open
/// chevron head is returned as an open path.
#[must_use]
pub fn arrow_head_paths(head: ArrowHead, tip: Vec2, from: Vec2, size: f64) -> Vec<Path> {
    if !head.is_visible() || size <= 0.0 {
        return Vec::new();
    }
    let direction = (tip - from).normalized();
    if direction.length_squared() <= f64::EPSILON {
        return Vec::new();
    }
    let normal = direction.perp();
    let back = tip - direction * size;
    let half = size * 0.55;
    match head {
        ArrowHead::None => Vec::new(),
        ArrowHead::Arrow => {
            let mut path = Path::new();
            path.move_to(tip);
            path.line_to(back + normal * half);
            path.move_to(tip);
            path.line_to(back - normal * half);
            vec![path]
        }
        ArrowHead::Triangle => vec![Path::polygon(&[
            tip,
            back + normal * half,
            back - normal * half,
        ])],
        ArrowHead::Diamond => {
            let mid = tip - direction * (size * 0.5);
            vec![Path::polygon(&[
                tip,
                mid + normal * half,
                back,
                mid - normal * half,
            ])]
        }
        ArrowHead::Square => {
            let mid = tip - direction * (size * 0.5);
            vec![Path::polygon(&[
                mid + normal * half,
                mid + direction * (size * 0.5) + normal * half,
                mid + direction * (size * 0.5) - normal * half,
                mid - normal * half,
            ])]
        }
        ArrowHead::Circle => vec![Path::ellipse(Rect::from_xywh(
            tip.x - size * 0.5,
            tip.y - size * 0.5,
            size,
            size,
        ))],
    }
}

/// Deterministic marker-like wobble for hand-drawn strokes.
#[must_use]
pub fn hand_drawn_path(path: &Path, seed: f64, jitter: f64) -> Path {
    let points = path.flatten(20);
    if points.len() < 2 {
        return path.clone();
    }
    let mut output = Path::new();
    for (index, point) in points.iter().enumerate() {
        let previous = points[index.saturating_sub(1)];
        let next = points[(index + 1).min(points.len() - 1)];
        let tangent = (next - previous).normalized();
        let normal = tangent.perp();
        let wobble = (index as f64 * 1.7 + seed).sin() * jitter
            + (index as f64 * 3.1 + seed * 0.7).cos() * jitter * 0.4;
        let value = *point + normal * wobble;
        if index == 0 {
            output.move_to(value);
        } else {
            output.line_to(value);
        }
    }
    output
}

/// Diagonal hatch lines across a rectangle, used for pattern fills on the CPU.
#[must_use]
pub fn pattern_lines(rect: Rect, spacing: f64) -> Vec<Path> {
    let spacing = spacing.max(3.0);
    let mut lines = Vec::new();
    if rect.is_empty() {
        return lines;
    }
    let mut offset = -rect.height();
    while offset < rect.width() + rect.height() {
        let mut path = Path::new();
        let start = Vec2::new(rect.min.x + offset, rect.max.y);
        let end = Vec2::new(rect.min.x + offset + rect.height(), rect.min.y);
        path.move_to(start);
        path.line_to(end);
        lines.push(path);
        offset += spacing;
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_geometric_kind_has_a_path() {
        for kind in ShapeKind::GEOMETRIC {
            let path = shape_path(kind, Rect::from_xywh(0.0, 0.0, 100.0, 60.0));
            assert!(!path.is_empty(), "{kind:?} produced an empty path");
        }
    }

    #[test]
    fn arrow_heads_have_geometry() {
        let paths = arrow_head_paths(ArrowHead::Triangle, Vec2::new(100.0, 0.0), Vec2::ZERO, 12.0);
        assert_eq!(paths.len(), 1);
        assert!(paths[0].bounds().width() > 0.0);
    }

    #[test]
    fn pattern_lines_are_bounded() {
        let lines = pattern_lines(Rect::from_xywh(0.0, 0.0, 50.0, 50.0), 8.0);
        assert!(!lines.is_empty());
    }
}
