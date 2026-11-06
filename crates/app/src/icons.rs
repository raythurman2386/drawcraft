//! Small original line icons, drawn with egui primitives.
//!
//! We draw icons instead of relying on symbol-font glyphs so the UI looks the
//! same on every platform and never falls back to tofu boxes.

use drawcraft_document::Rect as DocRect;
use drawcraft_editor::Tool;
use egui::{
    Align2, Color32, CornerRadius, FontId, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, pos2,
};

/// Paint a 16x16 icon in `rect`.
pub fn paint_tool_icon(painter: &Painter, rect: Rect, tool: Tool, color: Color32) {
    let stroke = Stroke::new(1.35, color);
    let point = |x: f32, y: f32| pos2(rect.left() + x, rect.top() + y);

    match tool {
        Tool::Select => {
            let points = vec![
                point(3.0, 1.0),
                point(3.0, 14.0),
                point(6.8, 10.2),
                point(9.4, 15.0),
                point(11.0, 14.2),
                point(8.5, 9.6),
                point(13.5, 9.6),
            ];
            painter.add(Shape::closed_line(points, stroke));
        }
        Tool::Lasso => {
            painter.add(Shape::closed_line(
                vec![
                    point(3.0, 5.0),
                    point(7.0, 2.5),
                    point(12.5, 4.0),
                    point(13.0, 10.0),
                    point(8.5, 13.5),
                    point(3.5, 11.0),
                ],
                stroke,
            ));
            painter.circle_filled(point(8.0, 8.0), 1.2, color);
        }
        Tool::Hand => {
            painter.add(Shape::line(
                vec![
                    point(4.0, 8.0),
                    point(4.0, 13.0),
                    point(12.0, 13.0),
                    point(12.0, 8.0),
                ],
                stroke,
            ));
            for x in [6.0, 8.0, 10.0] {
                painter.line_segment([point(x, 13.0), point(x, 6.0)], stroke);
            }
            painter.line_segment([point(6.0, 8.0), point(4.0, 6.0)], stroke);
            painter.line_segment([point(10.0, 8.0), point(12.0, 6.0)], stroke);
        }
        Tool::Draw => {
            painter.line_segment([point(3.0, 13.0), point(11.0, 5.0)], stroke);
            painter.line_segment([point(11.0, 5.0), point(13.5, 2.5)], stroke);
            painter.line_segment([point(11.0, 5.0), point(10.0, 2.5)], stroke);
        }
        Tool::Highlighter => {
            painter.line_segment(
                [point(3.0, 12.0), point(11.5, 3.5)],
                Stroke::new(5.0, color.gamma_multiply(0.35)),
            );
            painter.line_segment([point(3.0, 12.0), point(11.5, 3.5)], stroke);
            painter.line_segment([point(2.5, 15.0), point(10.0, 15.0)], stroke);
        }
        Tool::Eraser => {
            painter.rect_stroke(
                Rect::from_min_size(point(4.0, 4.0), egui::vec2(10.0, 7.0)),
                CornerRadius::same(2),
                stroke,
                StrokeKind::Inside,
            );
            painter.line_segment([point(4.0, 11.0), point(14.0, 11.0)], stroke);
        }
        Tool::Laser => {
            painter.circle_filled(point(5.0, 11.0), 2.0, color);
            for end in [point(1.0, 5.0), point(8.0, 2.0), point(14.0, 6.0)] {
                painter.line_segment([point(5.0, 11.0), end], stroke);
            }
        }
        Tool::Line => {
            painter.line_segment([point(3.0, 14.0), point(14.0, 3.0)], stroke);
        }
        Tool::Polyline => {
            painter.add(Shape::line(
                vec![
                    point(2.0, 12.0),
                    point(6.0, 5.0),
                    point(10.0, 11.0),
                    point(14.0, 3.0),
                ],
                stroke,
            ));
        }
        Tool::Arrow => {
            painter.line_segment([point(3.0, 13.0), point(13.0, 3.0)], stroke);
            painter.line_segment([point(7.0, 3.0), point(13.0, 3.0)], stroke);
            painter.line_segment([point(13.0, 3.0), point(13.0, 9.0)], stroke);
        }
        Tool::Text => {
            painter.text(
                point(8.0, 1.0),
                Align2::CENTER_TOP,
                "T",
                FontId::proportional(14.0),
                color,
            );
        }
        Tool::Sticky => {
            let points = vec![
                point(3.0, 2.0),
                point(11.0, 2.0),
                point(14.0, 5.0),
                point(14.0, 14.0),
                point(3.0, 14.0),
            ];
            painter.add(Shape::closed_line(points, stroke));
            painter.line_segment([point(11.0, 2.0), point(11.0, 5.0)], stroke);
            painter.line_segment([point(11.0, 5.0), point(14.0, 5.0)], stroke);
        }
        Tool::Note => {
            painter.rect_stroke(
                Rect::from_min_size(point(3.0, 2.0), egui::vec2(10.0, 12.0)),
                CornerRadius::same(1),
                stroke,
                StrokeKind::Inside,
            );
            for y in [6.0, 9.0, 12.0] {
                painter.line_segment([point(5.0, y), point(11.0, y)], stroke);
            }
        }
        Tool::Image => {
            painter.rect_stroke(
                Rect::from_min_size(point(2.0, 3.0), egui::vec2(12.0, 10.0)),
                CornerRadius::same(1),
                stroke,
                StrokeKind::Inside,
            );
            painter.add(Shape::line(
                vec![point(3.0, 12.0), point(6.5, 7.0), point(9.0, 10.0)],
                stroke,
            ));
            painter.circle_filled(point(11.0, 6.0), 1.2, color);
        }
        Tool::Frame => {
            for (x, y, dx, dy) in [
                (2.0, 2.0, 4.0, 0.0),
                (2.0, 2.0, 0.0, 4.0),
                (14.0, 2.0, -4.0, 0.0),
                (14.0, 2.0, 0.0, 4.0),
                (2.0, 14.0, 4.0, 0.0),
                (2.0, 14.0, 0.0, -4.0),
                (14.0, 14.0, -4.0, 0.0),
                (14.0, 14.0, 0.0, -4.0),
            ] {
                painter.line_segment([point(x, y), point(x + dx, y + dy)], stroke);
            }
        }
        _ => {
            if let Some(kind) = tool.shape_kind() {
                let rect = DocRect::from_xywh(
                    f64::from(rect.left() + 2.0),
                    f64::from(rect.top() + 2.0),
                    12.0,
                    12.0,
                );
                let path = drawcraft_render::shapes::shape_path(kind, rect);
                let points: Vec<Pos2> = path
                    .flatten(4)
                    .iter()
                    .map(|point| pos2(point.x as f32, point.y as f32))
                    .collect();
                if points.len() >= 2 {
                    painter.add(Shape::line(points, stroke));
                }
            }
        }
    }
}

/// The alignment commands exposed by the inspector.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AlignIcon {
    Left,
    Center,
    Right,
    Top,
    Middle,
    Bottom,
}

/// Paint a font-independent alignment icon: a reference edge plus aligned bars.
pub fn paint_align_icon(painter: &Painter, rect: Rect, icon: AlignIcon, color: Color32) {
    let stroke = Stroke::new(1.5, color);
    let point = |x: f32, y: f32| pos2(rect.left() + x, rect.top() + y);
    let bar = |x: f32, y: f32, width: f32, height: f32| {
        painter.rect_filled(
            Rect::from_min_size(point(x, y), egui::vec2(width, height)),
            CornerRadius::ZERO,
            color,
        );
    };
    match icon {
        AlignIcon::Left | AlignIcon::Center | AlignIcon::Right => {
            let x = match icon {
                AlignIcon::Left => 2.0,
                AlignIcon::Center => 8.0,
                AlignIcon::Right => 14.0,
                _ => unreachable!(),
            };
            painter.line_segment([point(x, 1.0), point(x, 15.0)], stroke);
            let (left_1, left_2, width) = match icon {
                AlignIcon::Left => (4.0, 4.0, 10.0),
                AlignIcon::Center => (3.0, 5.0, 10.0),
                AlignIcon::Right => (6.0, 9.0, 10.0),
                _ => unreachable!(),
            };
            bar(left_1, 4.0, width, 3.0);
            bar(left_2, 10.0, width - 4.0, 3.0);
        }
        AlignIcon::Top | AlignIcon::Middle | AlignIcon::Bottom => {
            let y = match icon {
                AlignIcon::Top => 2.0,
                AlignIcon::Middle => 8.0,
                AlignIcon::Bottom => 14.0,
                _ => unreachable!(),
            };
            painter.line_segment([point(1.0, y), point(15.0, y)], stroke);
            let (top_1, top_2, height) = match icon {
                AlignIcon::Top => (4.0, 4.0, 10.0),
                AlignIcon::Middle => (3.0, 6.0, 10.0),
                AlignIcon::Bottom => (2.0, 5.0, 10.0),
                _ => unreachable!(),
            };
            bar(4.0, top_1, 3.0, height);
            bar(10.0, top_2, 3.0, height - 4.0);
        }
    }
}
