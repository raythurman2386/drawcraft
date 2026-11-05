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
