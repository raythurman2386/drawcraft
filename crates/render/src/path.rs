use std::fmt::Write as _;

use drawcraft_geometry::{Rect, Transform, Vec2};

/// A path verb in world space.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Verb {
    MoveTo(Vec2),
    LineTo(Vec2),
    QuadTo(Vec2, Vec2),
    CubicTo(Vec2, Vec2, Vec2),
    Close,
}

/// A resolution-independent path made of line and Bézier segments.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Path {
    pub verbs: Vec<Verb>,
}

impl Path {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.verbs.is_empty()
    }

    pub fn move_to(&mut self, point: Vec2) {
        self.verbs.push(Verb::MoveTo(point));
    }

    pub fn line_to(&mut self, point: Vec2) {
        self.verbs.push(Verb::LineTo(point));
    }

    pub fn quad_to(&mut self, control: Vec2, point: Vec2) {
        self.verbs.push(Verb::QuadTo(control, point));
    }

    pub fn cubic_to(&mut self, c1: Vec2, c2: Vec2, point: Vec2) {
        self.verbs.push(Verb::CubicTo(c1, c2, point));
    }

    pub fn close(&mut self) {
        self.verbs.push(Verb::Close);
    }

    #[must_use]
    pub fn rect(rect: Rect) -> Self {
        let mut path = Self::new();
        path.move_to(rect.min);
        path.line_to(Vec2::new(rect.max.x, rect.min.y));
        path.line_to(rect.max);
        path.line_to(Vec2::new(rect.min.x, rect.max.y));
        path.close();
        path
    }

    #[must_use]
    pub fn rounded_rect(rect: Rect, radius: f64) -> Self {
        let radius = radius
            .min(rect.width() * 0.5)
            .min(rect.height() * 0.5)
            .max(0.0);
        if radius <= f64::EPSILON {
            return Self::rect(rect);
        }
        // Cubic approximation of a quarter circle.
        let control = radius * 0.552_284_8;
        let mut path = Self::new();
        path.move_to(Vec2::new(rect.min.x + radius, rect.min.y));
        path.line_to(Vec2::new(rect.max.x - radius, rect.min.y));
        path.cubic_to(
            Vec2::new(rect.max.x - radius + control, rect.min.y),
            Vec2::new(rect.max.x, rect.min.y + radius - control),
            Vec2::new(rect.max.x, rect.min.y + radius),
        );
        path.line_to(Vec2::new(rect.max.x, rect.max.y - radius));
        path.cubic_to(
            Vec2::new(rect.max.x, rect.max.y - radius + control),
            Vec2::new(rect.max.x - radius + control, rect.max.y),
            Vec2::new(rect.max.x - radius, rect.max.y),
        );
        path.line_to(Vec2::new(rect.min.x + radius, rect.max.y));
        path.cubic_to(
            Vec2::new(rect.min.x + radius - control, rect.max.y),
            Vec2::new(rect.min.x, rect.max.y - radius + control),
            Vec2::new(rect.min.x, rect.max.y - radius),
        );
        path.line_to(Vec2::new(rect.min.x, rect.min.y + radius));
        path.cubic_to(
            Vec2::new(rect.min.x, rect.min.y + radius - control),
            Vec2::new(rect.min.x + radius - control, rect.min.y),
            Vec2::new(rect.min.x + radius, rect.min.y),
        );
        path.close();
        path
    }

    #[must_use]
    pub fn ellipse(rect: Rect) -> Self {
        let rx = rect.width() * 0.5;
        let ry = rect.height() * 0.5;
        let center = rect.center();
        let kx = rx * 0.552_284_8;
        let ky = ry * 0.552_284_8;
        let mut path = Self::new();
        path.move_to(Vec2::new(center.x, rect.min.y));
        path.cubic_to(
            Vec2::new(center.x + kx, rect.min.y),
            Vec2::new(rect.max.x, center.y - ky),
            Vec2::new(rect.max.x, center.y),
        );
        path.cubic_to(
            Vec2::new(rect.max.x, center.y + ky),
            Vec2::new(center.x + kx, rect.max.y),
            Vec2::new(center.x, rect.max.y),
        );
        path.cubic_to(
            Vec2::new(center.x - kx, rect.max.y),
            Vec2::new(rect.min.x, center.y + ky),
            Vec2::new(rect.min.x, center.y),
        );
        path.cubic_to(
            Vec2::new(rect.min.x, center.y - ky),
            Vec2::new(center.x - kx, rect.min.y),
            Vec2::new(center.x, rect.min.y),
        );
        path.close();
        path
    }

    #[must_use]
    pub fn polygon(points: &[Vec2]) -> Self {
        let mut path = Self::new();
        if let Some((&first, rest)) = points.split_first() {
            path.move_to(first);
            for &point in rest {
                path.line_to(point);
            }
            path.close();
        }
        path
    }

    #[must_use]
    pub fn polyline(points: &[Vec2]) -> Self {
        let mut path = Self::new();
        if let Some((&first, rest)) = points.split_first() {
            path.move_to(first);
            for &point in rest {
                path.line_to(point);
            }
        }
        path
    }

    /// Regular polygon inscribed in `rect`.
    #[must_use]
    pub fn regular_polygon(rect: Rect, sides: usize) -> Self {
        let sides = sides.max(3);
        let center = rect.center();
        let rx = rect.width() * 0.5;
        let ry = rect.height() * 0.5;
        let start = -std::f64::consts::FRAC_PI_2;
        let points: Vec<Vec2> = (0..sides)
            .map(|index| {
                let angle = start + std::f64::consts::TAU * index as f64 / sides as f64;
                Vec2::new(center.x + rx * angle.cos(), center.y + ry * angle.sin())
            })
            .collect();
        Self::polygon(&points)
    }

    /// Five-pointed star inscribed in `rect`.
    #[must_use]
    pub fn star(rect: Rect, points: usize, inner_ratio: f64) -> Self {
        let points = points.max(3);
        let center = rect.center();
        let rx = rect.width() * 0.5;
        let ry = rect.height() * 0.5;
        let inner = inner_ratio.clamp(0.1, 0.95);
        let mut vertices = Vec::with_capacity(points * 2);
        for index in 0..points * 2 {
            let angle = -std::f64::consts::FRAC_PI_2
                + std::f64::consts::TAU * index as f64 / (points * 2) as f64;
            let radius = if index % 2 == 0 { 1.0 } else { inner };
            vertices.push(Vec2::new(
                center.x + rx * radius * angle.cos(),
                center.y + ry * radius * angle.sin(),
            ));
        }
        Self::polygon(&vertices)
    }

    #[must_use]
    pub fn heart(rect: Rect) -> Self {
        let mut path = Self::new();
        let center = rect.center();
        let w = rect.width();
        let h = rect.height();
        path.move_to(Vec2::new(center.x, rect.max.y));
        path.cubic_to(
            Vec2::new(rect.min.x - w * 0.1, center.y + h * 0.1),
            Vec2::new(rect.min.x + w * 0.1, rect.min.y - h * 0.1),
            Vec2::new(center.x, rect.min.y + h * 0.2),
        );
        path.cubic_to(
            Vec2::new(rect.max.x - w * 0.1, rect.min.y - h * 0.1),
            Vec2::new(rect.max.x + w * 0.1, center.y + h * 0.1),
            Vec2::new(center.x, rect.max.y),
        );
        path.close();
        path
    }

    #[must_use]
    pub fn cloud(rect: Rect) -> Self {
        let mut path = Self::new();
        let w = rect.width();
        let h = rect.height();
        let (x, y) = (rect.min.x, rect.min.y);
        path.move_to(Vec2::new(x + w * 0.22, y + h * 0.85));
        path.cubic_to(
            Vec2::new(x - w * 0.08, y + h * 0.82),
            Vec2::new(x + w * 0.02, y + h * 0.55),
            Vec2::new(x + w * 0.2, y + h * 0.58),
        );
        path.cubic_to(
            Vec2::new(x + w * 0.12, y + h * 0.25),
            Vec2::new(x + w * 0.4, y + h * 0.12),
            Vec2::new(x + w * 0.48, y + h * 0.32),
        );
        path.cubic_to(
            Vec2::new(x + w * 0.62, y + h * 0.02),
            Vec2::new(x + w * 0.92, y + h * 0.18),
            Vec2::new(x + w * 0.82, y + h * 0.48),
        );
        path.cubic_to(
            Vec2::new(x + w * 1.04, y + h * 0.58),
            Vec2::new(x + w * 0.9, y + h * 0.9),
            Vec2::new(x + w * 0.68, y + h * 0.85),
        );
        path.close();
        path
    }

    #[must_use]
    pub fn note(rect: Rect) -> Self {
        let fold = rect.width().min(rect.height()) * 0.18;
        let mut path = Self::new();
        path.move_to(rect.min);
        path.line_to(Vec2::new(rect.max.x - fold, rect.min.y));
        path.line_to(Vec2::new(rect.max.x, rect.min.y + fold));
        path.line_to(Vec2::new(rect.max.x, rect.max.y));
        path.line_to(Vec2::new(rect.min.x, rect.max.y));
        path.close();
        path
    }

    #[must_use]
    pub fn cylinder(rect: Rect) -> Self {
        let w = rect.width();
        let h = rect.height();
        let ry = (h * 0.14).min(w * 0.3);
        let mut path = Self::new();
        path.move_to(Vec2::new(rect.min.x, rect.min.y + ry));
        path.cubic_to(
            Vec2::new(rect.min.x, rect.min.y),
            Vec2::new(rect.max.x, rect.min.y),
            Vec2::new(rect.max.x, rect.min.y + ry),
        );
        path.line_to(Vec2::new(rect.max.x, rect.max.y - ry));
        path.cubic_to(
            Vec2::new(rect.max.x, rect.max.y),
            Vec2::new(rect.min.x, rect.max.y),
            Vec2::new(rect.min.x, rect.max.y - ry),
        );
        path.close();
        path
    }

    /// Apply a transform to every verb.
    pub fn transform(&mut self, transform: Transform) {
        for verb in &mut self.verbs {
            *verb = match *verb {
                Verb::MoveTo(point) => Verb::MoveTo(transform.apply(point)),
                Verb::LineTo(point) => Verb::LineTo(transform.apply(point)),
                Verb::QuadTo(control, point) => {
                    Verb::QuadTo(transform.apply(control), transform.apply(point))
                }
                Verb::CubicTo(c1, c2, point) => Verb::CubicTo(
                    transform.apply(c1),
                    transform.apply(c2),
                    transform.apply(point),
                ),
                Verb::Close => Verb::Close,
            };
        }
    }

    #[must_use]
    pub fn transformed(mut self, transform: Transform) -> Self {
        self.transform(transform);
        self
    }

    /// Bounding box of all path control points (conservative for curves).
    #[must_use]
    pub fn bounds(&self) -> Rect {
        let points: Vec<Vec2> = self
            .verbs
            .iter()
            .flat_map(|verb| match *verb {
                Verb::MoveTo(point) | Verb::LineTo(point) => vec![point],
                Verb::QuadTo(control, point) => vec![control, point],
                Verb::CubicTo(c1, c2, point) => vec![c1, c2, point],
                Verb::Close => Vec::new(),
            })
            .collect();
        Rect::from_points(&points)
    }

    /// Flatten curves into a polyline suitable for egui and hit testing.
    #[must_use]
    pub fn flatten(&self, segments_per_curve: usize) -> Vec<Vec2> {
        let steps = segments_per_curve.clamp(2, 64);
        let mut points = Vec::new();
        let mut current = Vec2::ZERO;
        let mut subpath_start = Vec2::ZERO;
        for verb in &self.verbs {
            match *verb {
                Verb::MoveTo(point) => {
                    points.push(point);
                    current = point;
                    subpath_start = point;
                }
                Verb::LineTo(point) => {
                    points.push(point);
                    current = point;
                }
                Verb::QuadTo(control, point) => {
                    let start = current;
                    for index in 1..=steps {
                        let t = index as f64 / steps as f64;
                        let one = 1.0 - t;
                        let value =
                            start * (one * one) + control * (2.0 * one * t) + point * (t * t);
                        points.push(value);
                    }
                    current = point;
                }
                Verb::CubicTo(c1, c2, point) => {
                    let start = current;
                    for index in 1..=steps {
                        let t = index as f64 / steps as f64;
                        let one = 1.0 - t;
                        let value = start * (one * one * one)
                            + c1 * (3.0 * one * one * t)
                            + c2 * (3.0 * one * t * t)
                            + point * (t * t * t);
                        points.push(value);
                    }
                    current = point;
                }
                Verb::Close => {
                    if points.last().copied() != Some(subpath_start) {
                        points.push(subpath_start);
                    }
                    current = subpath_start;
                }
            }
        }
        points
    }

    /// SVG `d` attribute.
    #[must_use]
    pub fn to_svg_d(&self) -> String {
        let mut output = String::new();
        for verb in &self.verbs {
            let _ = match verb {
                Verb::MoveTo(point) => write!(output, "M{:.3} {:.3} ", point.x, point.y),
                Verb::LineTo(point) => write!(output, "L{:.3} {:.3} ", point.x, point.y),
                Verb::QuadTo(control, point) => write!(
                    output,
                    "Q{:.3} {:.3} {:.3} {:.3} ",
                    control.x, control.y, point.x, point.y
                ),
                Verb::CubicTo(c1, c2, point) => write!(
                    output,
                    "C{:.3} {:.3} {:.3} {:.3} {:.3} {:.3} ",
                    c1.x, c1.y, c2.x, c2.y, point.x, point.y
                ),
                Verb::Close => write!(output, "Z "),
            };
        }
        output.trim_end().to_owned()
    }

    /// Split a flattened path into dash segments.
    #[must_use]
    pub fn dashed(&self, dash: f64, gap: f64) -> Vec<Path> {
        let points = self.flatten(24);
        if points.len() < 2 || dash <= 0.0 || gap < 0.0 {
            return vec![self.clone()];
        }
        let mut paths = Vec::new();
        let mut current = Path::new();
        let mut drawing = true;
        let mut remaining = dash;
        current.move_to(points[0]);
        for pair in points.windows(2) {
            let mut start = pair[0];
            let end = pair[1];
            let segment = (end - start).length();
            let mut consumed = 0.0;
            while consumed < segment - f64::EPSILON {
                let step = (segment - consumed).min(remaining);
                let next = start + (end - start).normalized() * step;
                if drawing {
                    current.line_to(next);
                } else {
                    current.move_to(next);
                }
                consumed += step;
                remaining -= step;
                if remaining <= f64::EPSILON {
                    if drawing {
                        paths.push(std::mem::take(&mut current));
                    }
                    drawing = !drawing;
                    remaining = if drawing { dash } else { gap };
                    if drawing {
                        current = Path::new();
                        current.move_to(next);
                    }
                }
                start = next;
            }
        }
        if drawing && current.verbs.len() > 1 {
            paths.push(current);
        }
        if paths.is_empty() {
            vec![self.clone()]
        } else {
            paths
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_has_five_verbs() {
        let path = Path::rect(Rect::from_xywh(0.0, 0.0, 10.0, 10.0));
        assert_eq!(path.verbs.len(), 5);
        assert_eq!(path.bounds(), Rect::from_xywh(0.0, 0.0, 10.0, 10.0));
    }

    #[test]
    fn note_has_folded_corner() {
        let rect = Rect::from_xywh(0.0, 0.0, 100.0, 80.0);
        let fold = 80.0 * 0.18;
        let path = Path::note(rect);
        let mut verbs = path.verbs.iter();
        assert_eq!(verbs.next(), Some(&Verb::MoveTo(Vec2::new(0.0, 0.0))));
        assert_eq!(
            verbs.next(),
            Some(&Verb::LineTo(Vec2::new(100.0 - fold, 0.0)))
        );
        // The corner is folded in: the third vertex lies down the right edge,
        // not at the top-right corner.
        assert_eq!(verbs.next(), Some(&Verb::LineTo(Vec2::new(100.0, fold))));
        assert_eq!(verbs.next(), Some(&Verb::LineTo(Vec2::new(100.0, 80.0))));
        assert_eq!(verbs.next(), Some(&Verb::LineTo(Vec2::new(0.0, 80.0))));
        assert_eq!(verbs.next(), Some(&Verb::Close));
        assert!(verbs.next().is_none());
    }

    #[test]
    fn ellipse_flattens_to_many_points() {
        let path = Path::ellipse(Rect::from_xywh(0.0, 0.0, 100.0, 50.0));
        assert!(path.flatten(8).len() > 20);
        assert!(path.to_svg_d().starts_with('M'));
    }

    #[test]
    fn dashed_splits_path() {
        let path = Path::polyline(&[Vec2::ZERO, Vec2::new(100.0, 0.0)]);
        assert!(path.dashed(10.0, 5.0).len() > 1);
    }
}
