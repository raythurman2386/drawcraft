use serde::{Deserialize, Serialize};

use crate::Vec2;

/// An axis-aligned rectangle.
///
/// `min` is always the top-left corner and `max` the bottom-right corner.
/// Constructors normalize reversed input so callers can pass any two corners.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub min: Vec2,
    pub max: Vec2,
}

impl Rect {
    /// The empty rectangle: contains nothing, unions with anything.
    pub const EMPTY: Self = Self {
        min: Vec2::new(f64::INFINITY, f64::INFINITY),
        max: Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
    };

    pub const ZERO: Self = Self {
        min: Vec2::ZERO,
        max: Vec2::ZERO,
    };

    #[must_use]
    pub fn new(a: Vec2, b: Vec2) -> Self {
        Self {
            min: a.min(b),
            max: a.max(b),
        }
    }

    #[must_use]
    pub fn from_xywh(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self::new(Vec2::new(x, y), Vec2::new(x + width, y + height))
    }

    /// Bounding box of a point cloud. Returns [`Rect::EMPTY`] for an empty slice.
    #[must_use]
    pub fn from_points(points: &[Vec2]) -> Self {
        points
            .iter()
            .copied()
            .fold(Self::EMPTY, |rect, point| Self {
                min: rect.min.min(point),
                max: rect.max.max(point),
            })
    }

    #[must_use]
    pub fn width(self) -> f64 {
        (self.max.x - self.min.x).max(0.0)
    }

    #[must_use]
    pub fn height(self) -> f64 {
        (self.max.y - self.min.y).max(0.0)
    }

    #[must_use]
    pub fn size(self) -> Vec2 {
        Vec2::new(self.width(), self.height())
    }

    #[must_use]
    pub fn center(self) -> Vec2 {
        (self.min + self.max) * 0.5
    }

    #[must_use]
    pub fn area(self) -> f64 {
        self.width() * self.height()
    }

    #[must_use]
    pub fn is_empty(self) -> bool {
        self.width() <= 0.0 || self.height() <= 0.0
    }

    #[must_use]
    pub fn contains(self, point: Vec2) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }

    #[must_use]
    pub fn contains_rect(self, other: Self) -> bool {
        self.contains(other.min) && self.contains(other.max)
    }

    #[must_use]
    pub fn intersects(self, other: Self) -> bool {
        !self.is_empty()
            && !other.is_empty()
            && self.max.x >= other.min.x
            && self.min.x <= other.max.x
            && self.max.y >= other.min.y
            && self.min.y <= other.max.y
    }

    #[must_use]
    pub fn intersection(self, other: Self) -> Self {
        let min = self.min.max(other.min);
        let max = self.max.min(other.max);
        if min.x > max.x || min.y > max.y {
            Self::EMPTY
        } else {
            Self { min, max }
        }
    }

    #[must_use]
    pub fn union(self, other: Self) -> Self {
        if self.is_empty() {
            return other;
        }
        if other.is_empty() {
            return self;
        }
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    #[must_use]
    pub fn expand(self, margin: f64) -> Self {
        if self.is_empty() {
            return self;
        }
        Self {
            min: self.min - Vec2::splat(margin),
            max: self.max + Vec2::splat(margin),
        }
    }

    #[must_use]
    pub fn translate(self, delta: Vec2) -> Self {
        if self.is_empty() {
            return self;
        }
        Self {
            min: self.min + delta,
            max: self.max + delta,
        }
    }

    #[must_use]
    pub fn corners(self) -> [Vec2; 4] {
        [
            self.min,
            Vec2::new(self.max.x, self.min.y),
            self.max,
            Vec2::new(self.min.x, self.max.y),
        ]
    }

    /// The rectangle after rotating its four corners around `center`.
    #[must_use]
    pub fn rotated_bounds(self, radians: f64, center: Vec2) -> Self {
        if radians.abs() <= f64::EPSILON {
            return self;
        }
        let corners = self.corners().map(|point| {
            let delta = point - center;
            let (sin, cos) = radians.sin_cos();
            center + Vec2::new(delta.x * cos - delta.y * sin, delta.x * sin + delta.y * cos)
        });
        Self::from_points(&corners)
    }
}

impl Default for Rect {
    fn default() -> Self {
        Self::ZERO
    }
}
