use serde::{Deserialize, Serialize};

use crate::{Rect, Vec2};

/// A 2D affine transform stored as a 2x3 matrix.
///
/// Points are column vectors: `p' = M * p`.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    pub m11: f64,
    pub m12: f64,
    pub m21: f64,
    pub m22: f64,
    pub tx: f64,
    pub ty: f64,
}

impl Transform {
    pub const IDENTITY: Self = Self {
        m11: 1.0,
        m12: 0.0,
        m21: 0.0,
        m22: 1.0,
        tx: 0.0,
        ty: 0.0,
    };

    #[must_use]
    pub const fn translate(offset: Vec2) -> Self {
        Self {
            m11: 1.0,
            m12: 0.0,
            m21: 0.0,
            m22: 1.0,
            tx: offset.x,
            ty: offset.y,
        }
    }

    #[must_use]
    pub const fn scale(factor: Vec2) -> Self {
        Self {
            m11: factor.x,
            m12: 0.0,
            m21: 0.0,
            m22: factor.y,
            tx: 0.0,
            ty: 0.0,
        }
    }

    #[must_use]
    pub fn rotation(radians: f64) -> Self {
        let (sin, cos) = radians.sin_cos();
        Self {
            m11: cos,
            m12: -sin,
            m21: sin,
            m22: cos,
            tx: 0.0,
            ty: 0.0,
        }
    }

    #[must_use]
    pub fn rotation_about(radians: f64, center: Vec2) -> Self {
        Self::translate(center) * Self::rotation(radians) * Self::translate(-center)
    }

    /// Apply this transform to a point.
    #[must_use]
    pub fn apply(self, point: Vec2) -> Vec2 {
        Vec2::new(
            self.m11 * point.x + self.m21 * point.y + self.tx,
            self.m12 * point.x + self.m22 * point.y + self.ty,
        )
    }

    /// Apply this transform to a direction (ignores translation).
    #[must_use]
    pub fn apply_vector(self, vector: Vec2) -> Vec2 {
        Vec2::new(
            self.m11 * vector.x + self.m21 * vector.y,
            self.m12 * vector.x + self.m22 * vector.y,
        )
    }

    /// Matrix multiplication: `self * rhs` applies `rhs` first, then `self`.
    #[must_use]
    pub fn then(self, next: Self) -> Self {
        next * self
    }

    #[must_use]
    pub fn inverse(self) -> Option<Self> {
        let det = self.m11 * self.m22 - self.m12 * self.m21;
        if det.abs() <= f64::EPSILON {
            return None;
        }
        let inv = 1.0 / det;
        Some(Self {
            m11: self.m22 * inv,
            m12: -self.m12 * inv,
            m21: -self.m21 * inv,
            m22: self.m11 * inv,
            tx: (self.m21 * self.ty - self.m22 * self.tx) * inv,
            ty: (self.m12 * self.tx - self.m11 * self.ty) * inv,
        })
    }

    /// Bounding box of a transformed rectangle.
    #[must_use]
    pub fn apply_rect(self, rect: Rect) -> Rect {
        Rect::from_points(&rect.corners().map(|point| self.apply(point)))
    }
}

impl std::ops::Mul for Transform {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self {
            m11: self.m11 * rhs.m11 + self.m21 * rhs.m12,
            m12: self.m12 * rhs.m11 + self.m22 * rhs.m12,
            m21: self.m11 * rhs.m21 + self.m21 * rhs.m22,
            m22: self.m12 * rhs.m21 + self.m22 * rhs.m22,
            tx: self.m11 * rhs.tx + self.m21 * rhs.ty + self.tx,
            ty: self.m12 * rhs.tx + self.m22 * rhs.ty + self.ty,
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}
