use serde::{Deserialize, Serialize};

use crate::{Rect, Vec2, clamp};

/// Minimum and maximum zoom factors accepted by the camera.
pub const MIN_ZOOM: f64 = 0.02;
pub const MAX_ZOOM: f64 = 64.0;

/// A 2D camera looking at world space.
///
/// Coordinates are viewport-relative: `(0, 0)` is the top-left of the canvas
/// and `viewport` is its size in logical pixels.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Camera {
    pub center: Vec2,
    pub zoom: f64,
}

impl Camera {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            center: Vec2::ZERO,
            zoom: 1.0,
        }
    }

    #[must_use]
    pub fn with_zoom(mut self, zoom: f64) -> Self {
        self.zoom = clamp(zoom, MIN_ZOOM, MAX_ZOOM);
        self
    }

    #[must_use]
    pub fn screen_to_world(self, point: Vec2, viewport: Vec2) -> Vec2 {
        self.center + (point - viewport * 0.5) / self.zoom
    }

    #[must_use]
    pub fn world_to_screen(self, point: Vec2, viewport: Vec2) -> Vec2 {
        viewport * 0.5 + (point - self.center) * self.zoom
    }

    /// Convert a screen-space delta into a world-space delta.
    #[must_use]
    pub fn screen_delta_to_world(self, delta: Vec2) -> Vec2 {
        delta / self.zoom
    }

    /// Pan by a screen-space delta (e.g. a mouse drag).
    pub fn pan(&mut self, screen_delta: Vec2) {
        self.center -= self.screen_delta_to_world(screen_delta);
    }

    /// Zoom around a fixed viewport point, keeping the world point under the
    /// cursor stationary.
    pub fn zoom_at(&mut self, cursor: Vec2, viewport: Vec2, factor: f64) {
        if !factor.is_finite() || factor <= 0.0 {
            return;
        }
        let world_before = self.screen_to_world(cursor, viewport);
        self.zoom = clamp(self.zoom * factor, MIN_ZOOM, MAX_ZOOM);
        self.center = world_before - (cursor - viewport * 0.5) / self.zoom;
    }

    /// Set an absolute zoom while keeping `cursor` anchored.
    pub fn set_zoom_at(&mut self, cursor: Vec2, viewport: Vec2, zoom: f64) {
        let target = clamp(zoom, MIN_ZOOM, MAX_ZOOM);
        if self.zoom.abs() <= f64::EPSILON {
            self.zoom = target;
            return;
        }
        self.zoom_at(cursor, viewport, target / self.zoom);
    }

    /// Fit `bounds` into the viewport with a margin expressed as a fraction of
    /// the viewport size (0.1 = 10%).
    pub fn zoom_to_fit(&mut self, bounds: Rect, viewport: Vec2, margin: f64) {
        if bounds.is_empty() || viewport.x <= 1.0 || viewport.y <= 1.0 {
            return;
        }
        let available = viewport * (1.0 - 2.0 * clamp(margin, 0.0, 0.45));
        if available.x <= 1.0 || available.y <= 1.0 {
            return;
        }
        let scale = (available.x / bounds.width()).min(available.y / bounds.height());
        self.zoom = clamp(scale, MIN_ZOOM, MAX_ZOOM);
        self.center = bounds.center();
    }

    #[must_use]
    pub fn visible_world_rect(self, viewport: Vec2) -> Rect {
        let half = viewport * 0.5 / self.zoom;
        Rect::new(self.center - half, self.center + half)
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new()
    }
}
