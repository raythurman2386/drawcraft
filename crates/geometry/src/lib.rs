//! Pure 2D geometry for DrawCraft.
//!
//! This crate is deliberately dependency-light (serde only) and free of I/O so
//! that every rule about coordinates can be tested without a window. All public
//! operations are total: non-finite inputs are handled by returning the input
//! unchanged or a documented zero value rather than panicking.

#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::field_reassign_with_default
    )
)]

mod camera;
mod math;
mod rect;
mod transform;
mod vec2;

pub use camera::Camera;
pub use math::{
    clamp, distance_point_segment, point_in_polygon, rect_in_polygon, simplify_polyline,
    smooth_polyline,
};
pub use rect::Rect;
pub use transform::Transform;
pub use vec2::Vec2;

/// Tolerance used when comparing floating point geometry.
pub const EPSILON: f64 = 1.0e-9;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_round_trips_world_points() {
        let camera = Camera::new().with_zoom(2.0);
        let viewport = Vec2::new(800.0, 600.0);
        let world = Vec2::new(123.0, -45.0);
        let screen = camera.world_to_screen(world, viewport);
        let round_trip = camera.screen_to_world(screen, viewport);
        assert!((round_trip - world).length() < 1.0e-9);
    }

    #[test]
    fn zoom_keeps_cursor_world_position_fixed() {
        let mut camera = Camera::new();
        let viewport = Vec2::new(1000.0, 700.0);
        let cursor = Vec2::new(250.0, 400.0);
        let before = camera.screen_to_world(cursor, viewport);
        camera.zoom_at(cursor, viewport, 1.75);
        let after = camera.screen_to_world(cursor, viewport);
        assert!((before - after).length() < 1.0e-9);
    }

    #[test]
    fn rect_union_and_intersection() {
        let a = Rect::from_xywh(0.0, 0.0, 10.0, 10.0);
        let b = Rect::from_xywh(5.0, 5.0, 10.0, 10.0);
        assert!(a.intersects(b));
        let union = a.union(b);
        assert_eq!(union, Rect::from_xywh(0.0, 0.0, 15.0, 15.0));
    }

    #[test]
    fn point_in_polygon_handles_concave_shapes() {
        let polygon = [
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(5.0, 4.0),
            Vec2::new(0.0, 10.0),
        ];
        assert!(point_in_polygon(Vec2::new(2.0, 2.0), &polygon));
        assert!(!point_in_polygon(Vec2::new(5.0, 9.0), &polygon));
    }

    #[test]
    fn rect_in_polygon_requires_full_containment() {
        let polygon = [
            Vec2::new(0.0, 0.0),
            Vec2::new(100.0, 0.0),
            Vec2::new(100.0, 100.0),
            Vec2::new(0.0, 100.0),
        ];
        let inside = Rect::from_xywh(10.0, 10.0, 20.0, 20.0);
        let overhang = Rect::from_xywh(90.0, 90.0, 20.0, 20.0);
        assert!(rect_in_polygon(inside, &polygon));
        assert!(!rect_in_polygon(overhang, &polygon));
    }

    #[test]
    fn simplify_keeps_endpoints() {
        let points = [
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.1),
            Vec2::new(2.0, 0.0),
            Vec2::new(3.0, 0.0),
        ];
        let simplified = simplify_polyline(&points, 0.5);
        assert_eq!(simplified.first(), Some(&points[0]));
        assert_eq!(simplified.last(), Some(&points[3]));
    }
}
