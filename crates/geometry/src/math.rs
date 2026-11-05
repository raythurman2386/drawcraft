use crate::Vec2;

/// Clamp with a total fallback for non-finite input.
#[must_use]
pub fn clamp(value: f64, min: f64, max: f64) -> f64 {
    if value.is_nan() {
        min
    } else {
        value.clamp(min, max)
    }
}

/// Shortest distance from `point` to the segment `a..b`.
#[must_use]
pub fn distance_point_segment(point: Vec2, a: Vec2, b: Vec2) -> f64 {
    let ab = b - a;
    let length_squared = ab.length_squared();
    if length_squared <= f64::EPSILON {
        return point.distance(a);
    }
    let t = clamp((point - a).dot(ab) / length_squared, 0.0, 1.0);
    point.distance(a + ab * t)
}

/// Even-odd point-in-polygon test. The polygon is implicitly closed.
#[must_use]
pub fn point_in_polygon(point: Vec2, polygon: &[Vec2]) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut previous = polygon[polygon.len() - 1];
    for &current in polygon {
        let crosses = (current.y > point.y) != (previous.y > point.y);
        if crosses {
            let denominator = previous.y - current.y;
            if denominator.abs() > f64::EPSILON {
                let x = current.x + (point.y - current.y) * (previous.x - current.x) / denominator;
                if point.x < x {
                    inside = !inside;
                }
            }
        }
        previous = current;
    }
    inside
}

/// True when every corner of `rect` lies inside `polygon` (fully contained bounds).
///
/// Empty rectangles are treated as a single point at `min`. Used by lasso
/// selection so the lasso is a precision contain tool while marquee stays touch.
#[must_use]
pub fn rect_in_polygon(rect: crate::Rect, polygon: &[Vec2]) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    if rect.is_empty() {
        return point_in_polygon(rect.min, polygon);
    }
    rect.corners()
        .into_iter()
        .all(|corner| point_in_polygon(corner, polygon))
}

/// Ramer-Douglas-Peucker polyline simplification.
///
/// Iterative so that a hostile document with many points cannot overflow the
/// stack. Returns a new vector with the same endpoints.
#[must_use]
pub fn simplify_polyline(points: &[Vec2], epsilon: f64) -> Vec<Vec2> {
    if points.len() <= 2 || !epsilon.is_finite() || epsilon <= 0.0 {
        return points.to_vec();
    }

    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[points.len() - 1] = true;

    let mut stack = vec![(0usize, points.len() - 1)];
    while let Some((start, end)) = stack.pop() {
        if end <= start + 1 {
            continue;
        }
        let mut farthest = start;
        let mut farthest_distance = 0.0;
        let a = points[start];
        let b = points[end];
        for (offset, &point) in points[start + 1..end].iter().enumerate() {
            let distance = distance_point_segment(point, a, b);
            if distance > farthest_distance {
                farthest_distance = distance;
                farthest = start + 1 + offset;
            }
        }
        if farthest_distance > epsilon {
            keep[farthest] = true;
            stack.push((start, farthest));
            stack.push((farthest, end));
        }
    }

    points
        .iter()
        .zip(keep)
        .filter_map(|(&point, keep)| keep.then_some(point))
        .collect()
}

/// Chaikin-style smoothing that keeps the original endpoints.
#[must_use]
pub fn smooth_polyline(points: &[Vec2], iterations: usize) -> Vec<Vec2> {
    if points.len() < 3 || iterations == 0 {
        return points.to_vec();
    }
    let mut current = points.to_vec();
    for _ in 0..iterations.min(4) {
        let mut next = Vec::with_capacity(current.len() * 2);
        if let Some(&first) = current.first() {
            next.push(first);
        }
        for pair in current.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            next.push(a.lerp(b, 0.25));
            next.push(a.lerp(b, 0.75));
        }
        if let Some(&last) = current.last() {
            next.push(last);
        }
        current = next;
    }
    current
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_to_degenerate_segment() {
        let d = distance_point_segment(Vec2::new(3.0, 4.0), Vec2::ZERO, Vec2::ZERO);
        assert!((d - 5.0).abs() < 1.0e-9);
    }

    #[test]
    fn smooth_keeps_endpoints() {
        let points = [Vec2::ZERO, Vec2::new(5.0, 5.0), Vec2::new(10.0, 0.0)];
        let smoothed = smooth_polyline(&points, 2);
        assert_eq!(smoothed.first(), Some(&points[0]));
        assert_eq!(smoothed.last(), Some(&points[2]));
        assert!(smoothed.len() > points.len());
    }
}
