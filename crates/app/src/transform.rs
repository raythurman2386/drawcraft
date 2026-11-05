//! Pure transform-gesture math for the canvas resize / rotate handles.
//!
//! Kept free of egui so modifier behaviour (Shift aspect lock, Alt centre
//! anchor, Shift 15° rotation snap) is unit-testable. Headless `edit.resize` /
//! `edit.rotate` do not use these helpers; they are gesture-only.

use drawcraft_document::{Rect, Vec2};

use crate::state::ResizeHandle;

/// Rotation snap increment used while Shift is held (15°).
pub const ROTATE_SNAP_STEP: f64 = std::f64::consts::PI / 12.0;

const MIN_SCALE: f64 = 0.02;
const MAX_SCALE: f64 = 50.0;

/// Modifier flags read once per drag frame.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct ResizeModifiers {
    /// Shift: keep the start bounds' aspect ratio.
    pub keep_aspect: bool,
    /// Alt: scale symmetrically about the selection centre.
    pub from_centre: bool,
}

/// Anchor point and per-axis scale for dragging `handle` of `bounds` to `pointer`.
#[must_use]
pub fn resize_scale(
    handle: ResizeHandle,
    bounds: Rect,
    pointer: Vec2,
    modifiers: ResizeModifiers,
) -> (Vec2, Vec2) {
    let width = bounds.width().max(1.0);
    let height = bounds.height().max(1.0);
    let centre = Vec2::new(
        (bounds.min.x + bounds.max.x) * 0.5,
        (bounds.min.y + bounds.max.y) * 0.5,
    );
    let mut anchor = Vec2::new(
        if handle.is_west() {
            bounds.max.x
        } else {
            bounds.min.x
        },
        if handle.is_north() {
            bounds.max.y
        } else {
            bounds.min.y
        },
    );
    // From centre the dragged edge moves half the span per unit of scale.
    let span = if modifiers.from_centre { 0.5 } else { 1.0 };
    if modifiers.from_centre {
        anchor = centre;
    }
    let horizontal = handle.is_east() || handle.is_west();
    let vertical = handle.is_north() || handle.is_south();
    let mut scale_x = if handle.is_east() {
        (pointer.x - anchor.x) / (width * span)
    } else if handle.is_west() {
        (anchor.x - pointer.x) / (width * span)
    } else {
        1.0
    };
    let mut scale_y = if handle.is_south() {
        (pointer.y - anchor.y) / (height * span)
    } else if handle.is_north() {
        (anchor.y - pointer.y) / (height * span)
    } else {
        1.0
    };
    if modifiers.keep_aspect {
        let uniform = if horizontal && vertical {
            // Corner: follow whichever axis moved further from 1.
            if (scale_x - 1.0).abs() >= (scale_y - 1.0).abs() {
                scale_x
            } else {
                scale_y
            }
        } else if horizontal {
            scale_x
        } else {
            scale_y
        };
        scale_x = uniform;
        scale_y = uniform;
        // Edge handles grow the free axis symmetrically about the centre line.
        if !vertical {
            anchor.y = centre.y;
        }
        if !horizontal {
            anchor.x = centre.x;
        }
    }
    (
        anchor,
        Vec2::new(
            scale_x.clamp(MIN_SCALE, MAX_SCALE),
            scale_y.clamp(MIN_SCALE, MAX_SCALE),
        ),
    )
}

/// Snap `angle` (radians) to the nearest multiple of [`ROTATE_SNAP_STEP`] when `snap`.
#[must_use]
pub fn rotate_angle(angle: f64, snap: bool) -> f64 {
    if snap {
        (angle / ROTATE_SNAP_STEP).round() * ROTATE_SNAP_STEP
    } else {
        angle
    }
}

/// Degrees in (-180, 180] for display.
#[must_use]
pub fn display_degrees(angle: f64) -> f64 {
    let mut deg = angle.to_degrees() % 360.0;
    if deg > 180.0 {
        deg -= 360.0;
    } else if deg <= -180.0 {
        deg += 360.0;
    }
    if deg.abs() < 0.05 { 0.0 } else { deg }
}

/// Live numeric readout text: `W × H` plus rotation degrees.
#[must_use]
pub fn readout(size: Vec2, angle: f64) -> String {
    format!(
        "{:.0} × {:.0}  {:.0}°",
        size.x,
        size.y,
        display_degrees(angle)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect() -> Rect {
        Rect::from_xywh(0.0, 0.0, 200.0, 100.0)
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn plain_corner_is_free_and_anchors_opposite_corner() {
        let (anchor, scale) = resize_scale(
            ResizeHandle::SouthEast,
            rect(),
            Vec2::new(400.0, 150.0),
            ResizeModifiers::default(),
        );
        assert_eq!((anchor.x, anchor.y), (0.0, 0.0));
        assert!(close(scale.x, 2.0) && close(scale.y, 1.5));
    }

    #[test]
    fn shift_corner_locks_aspect() {
        let mods = ResizeModifiers {
            keep_aspect: true,
            from_centre: false,
        };
        let (anchor, scale) = resize_scale(
            ResizeHandle::SouthEast,
            rect(),
            Vec2::new(400.0, 150.0),
            mods,
        );
        assert_eq!((anchor.x, anchor.y), (0.0, 0.0));
        assert!(close(scale.x, 2.0) && close(scale.y, 2.0));
        let (anchor, scale) = resize_scale(
            ResizeHandle::NorthWest,
            rect(),
            Vec2::new(150.0, -100.0),
            mods,
        );
        assert_eq!((anchor.x, anchor.y), (200.0, 100.0));
        assert!(close(scale.x, 2.0) && close(scale.y, 2.0));
    }

    #[test]
    fn shift_edge_scales_both_axes_about_centre_line() {
        let mods = ResizeModifiers {
            keep_aspect: true,
            from_centre: false,
        };
        let (anchor, scale) =
            resize_scale(ResizeHandle::East, rect(), Vec2::new(300.0, 999.0), mods);
        assert!(close(scale.x, 1.5) && close(scale.y, 1.5));
        assert_eq!((anchor.x, anchor.y), (0.0, 50.0));
    }

    #[test]
    fn alt_scales_from_centre() {
        let mods = ResizeModifiers {
            keep_aspect: false,
            from_centre: true,
        };
        let (anchor, scale) = resize_scale(ResizeHandle::East, rect(), Vec2::new(250.0, 0.0), mods);
        assert_eq!((anchor.x, anchor.y), (100.0, 50.0));
        // Edge moved 50 right of 200 -> half-width 150 vs 100.
        assert!(close(scale.x, 1.5) && close(scale.y, 1.0));
        let (anchor, scale) = resize_scale(
            ResizeHandle::NorthWest,
            rect(),
            Vec2::new(-100.0, 0.0),
            mods,
        );
        assert_eq!((anchor.x, anchor.y), (100.0, 50.0));
        assert!(close(scale.x, 2.0) && close(scale.y, 1.0));
    }

    #[test]
    fn shift_alt_corner_is_uniform_about_centre() {
        let mods = ResizeModifiers {
            keep_aspect: true,
            from_centre: true,
        };
        let (anchor, scale) = resize_scale(
            ResizeHandle::SouthEast,
            rect(),
            Vec2::new(300.0, 110.0),
            mods,
        );
        assert_eq!((anchor.x, anchor.y), (100.0, 50.0));
        assert!(close(scale.x, 2.0) && close(scale.y, 2.0));
    }

    #[test]
    fn scale_is_clamped() {
        let (_, scale) = resize_scale(
            ResizeHandle::East,
            rect(),
            Vec2::new(-500.0, 0.0),
            ResizeModifiers::default(),
        );
        assert!(close(scale.x, MIN_SCALE));
    }

    #[test]
    fn rotate_snaps_to_15_degrees_only_with_shift() {
        let raw = 22.0_f64.to_radians();
        assert!(close(rotate_angle(raw, false), raw));
        assert!(close(rotate_angle(raw, true), 15.0_f64.to_radians()));
        assert!(close(
            rotate_angle(23.0_f64.to_radians(), true),
            30.0_f64.to_radians()
        ));
        assert!(close(
            rotate_angle(-8.0_f64.to_radians(), true),
            -15.0_f64.to_radians()
        ));
        assert!(close(rotate_angle(3.0_f64.to_radians(), true), 0.0));
    }

    #[test]
    fn readout_shows_size_and_degrees() {
        assert_eq!(
            readout(Vec2::new(200.4, 99.6), 45.0_f64.to_radians()),
            "200 × 100  45°"
        );
        assert!(close(display_degrees(270.0_f64.to_radians()), -90.0));
        assert_eq!(readout(Vec2::new(10.0, 10.0), 0.0), "10 × 10  0°");
    }
}
