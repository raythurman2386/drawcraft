use drawcraft_geometry::Vec2;

/// Ear-clipping triangulation for a simple polygon.
///
/// Returns triangle indices into `points`. The input is treated as closed and
/// duplicate closing points are ignored. This is used by the GPU UI to fill
/// concave shapes correctly; SVG export uses path fill rules instead.
#[must_use]
pub fn triangulate(points: &[Vec2]) -> Vec<[u32; 3]> {
    let mut polygon = points.to_vec();
    if polygon.len() > 1 && polygon[0].distance(polygon[polygon.len() - 1]) < 1.0e-9 {
        polygon.pop();
    }
    let len = polygon.len();
    if len < 3 {
        return Vec::new();
    }
    // Ensure counter-clockwise winding.
    if signed_area(&polygon) < 0.0 {
        polygon.reverse();
    }

    let mut indices: Vec<usize> = (0..len).collect();
    let mut triangles = Vec::with_capacity(len.saturating_sub(2));
    let mut guard = 0usize;
    while indices.len() > 3 && guard < len * len {
        guard += 1;
        let mut clipped = false;
        for index in 0..indices.len() {
            let previous = indices[(index + indices.len() - 1) % indices.len()];
            let current = indices[index];
            let next = indices[(index + 1) % indices.len()];
            let (a, b, c) = (polygon[previous], polygon[current], polygon[next]);
            if cross(b - a, c - b) <= 0.0 {
                continue;
            }
            let mut contains = false;
            for &other in &indices {
                if other == previous || other == current || other == next {
                    continue;
                }
                if point_in_triangle(polygon[other], a, b, c) {
                    contains = true;
                    break;
                }
            }
            if contains {
                continue;
            }
            triangles.push([previous as u32, current as u32, next as u32]);
            indices.remove(index);
            clipped = true;
            break;
        }
        if !clipped {
            break;
        }
    }
    if indices.len() == 3 {
        triangles.push([indices[0] as u32, indices[1] as u32, indices[2] as u32]);
    }
    triangles
}

fn signed_area(points: &[Vec2]) -> f64 {
    let mut area = 0.0;
    for index in 0..points.len() {
        let a = points[index];
        let b = points[(index + 1) % points.len()];
        area += a.cross(b);
    }
    area * 0.5
}

fn cross(a: Vec2, b: Vec2) -> f64 {
    a.cross(b)
}

fn point_in_triangle(point: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
    let d1 = (point - a).cross(b - a);
    let d2 = (point - b).cross(c - b);
    let d3 = (point - c).cross(a - c);
    let negative = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let positive = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(negative && positive)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triangulates_convex_polygon() {
        let points = [
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(0.0, 10.0),
        ];
        assert_eq!(triangulate(&points).len(), 2);
    }

    #[test]
    fn triangulates_concave_polygon() {
        let points = [
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(5.0, 4.0),
            Vec2::new(0.0, 10.0),
        ];
        assert_eq!(triangulate(&points).len(), 3);
    }

    #[test]
    fn rejects_degenerate_polygon() {
        assert!(triangulate(&[Vec2::ZERO, Vec2::X]).is_empty());
    }
}
