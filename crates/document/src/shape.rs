use std::collections::BTreeMap;

use drawcraft_geometry::{Rect, Transform, Vec2};
use serde::{Deserialize, Serialize};

use crate::style::Style;
use crate::{Dash, GroupId, ShapeId};

/// Every kind of record DrawCraft can place on a page.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShapeKind {
    #[default]
    Rectangle,
    Ellipse,
    Triangle,
    Diamond,
    Pentagon,
    Hexagon,
    Octagon,
    Star,
    Heart,
    RoundRect,
    Parallelogram,
    Trapezoid,
    ArrowShape,
    DoubleArrow,
    Cross,
    HomePlate,
    Chevron,
    Cylinder,
    Cloud,
    Note,
    Text,
    Sticky,
    Image,
    Frame,
    Draw,
    Highlighter,
    Line,
    Polyline,
    Arrow,
    Laser,
    Group,
}

impl ShapeKind {
    pub const ALL: [Self; 31] = [
        Self::Rectangle,
        Self::Ellipse,
        Self::Triangle,
        Self::Diamond,
        Self::Pentagon,
        Self::Hexagon,
        Self::Octagon,
        Self::Star,
        Self::Heart,
        Self::RoundRect,
        Self::Parallelogram,
        Self::Trapezoid,
        Self::ArrowShape,
        Self::DoubleArrow,
        Self::Cross,
        Self::HomePlate,
        Self::Chevron,
        Self::Cylinder,
        Self::Cloud,
        Self::Note,
        Self::Text,
        Self::Sticky,
        Self::Image,
        Self::Frame,
        Self::Draw,
        Self::Highlighter,
        Self::Line,
        Self::Polyline,
        Self::Arrow,
        Self::Laser,
        Self::Group,
    ];

    /// The 20 geometric presets exposed in the tool rail.
    pub const GEOMETRIC: [Self; 20] = [
        Self::Rectangle,
        Self::Ellipse,
        Self::Triangle,
        Self::Diamond,
        Self::Pentagon,
        Self::Hexagon,
        Self::Octagon,
        Self::Star,
        Self::Heart,
        Self::RoundRect,
        Self::Parallelogram,
        Self::Trapezoid,
        Self::ArrowShape,
        Self::DoubleArrow,
        Self::Cross,
        Self::HomePlate,
        Self::Chevron,
        Self::Cylinder,
        Self::Cloud,
        Self::Note,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Rectangle => "rectangle",
            Self::Ellipse => "ellipse",
            Self::Triangle => "triangle",
            Self::Diamond => "diamond",
            Self::Pentagon => "pentagon",
            Self::Hexagon => "hexagon",
            Self::Octagon => "octagon",
            Self::Star => "star",
            Self::Heart => "heart",
            Self::RoundRect => "round_rect",
            Self::Parallelogram => "parallelogram",
            Self::Trapezoid => "trapezoid",
            Self::ArrowShape => "arrow_shape",
            Self::DoubleArrow => "double_arrow",
            Self::Cross => "cross",
            Self::HomePlate => "home_plate",
            Self::Chevron => "chevron",
            Self::Cylinder => "cylinder",
            Self::Cloud => "cloud",
            Self::Note => "note",
            Self::Text => "text",
            Self::Sticky => "sticky",
            Self::Image => "image",
            Self::Frame => "frame",
            Self::Draw => "draw",
            Self::Highlighter => "highlighter",
            Self::Line => "line",
            Self::Polyline => "polyline",
            Self::Arrow => "arrow",
            Self::Laser => "laser",
            Self::Group => "group",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "rectangle" | "rect" => Self::Rectangle,
            "ellipse" => Self::Ellipse,
            "triangle" => Self::Triangle,
            "diamond" => Self::Diamond,
            "pentagon" => Self::Pentagon,
            "hexagon" => Self::Hexagon,
            "octagon" => Self::Octagon,
            "star" => Self::Star,
            "heart" => Self::Heart,
            "round_rect" | "roundrect" => Self::RoundRect,
            "parallelogram" => Self::Parallelogram,
            "trapezoid" => Self::Trapezoid,
            "arrow_shape" | "arrowshape" => Self::ArrowShape,
            "double_arrow" | "doublearrow" => Self::DoubleArrow,
            "cross" => Self::Cross,
            "home_plate" | "homeplate" => Self::HomePlate,
            "chevron" => Self::Chevron,
            "cylinder" => Self::Cylinder,
            "cloud" => Self::Cloud,
            "note" => Self::Note,
            "text" => Self::Text,
            "sticky" => Self::Sticky,
            "image" => Self::Image,
            "frame" => Self::Frame,
            "draw" => Self::Draw,
            "highlighter" => Self::Highlighter,
            "line" => Self::Line,
            "polyline" => Self::Polyline,
            "arrow" => Self::Arrow,
            "laser" => Self::Laser,
            "group" => Self::Group,
            _ => return None,
        })
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Rectangle => "Rectangle",
            Self::Ellipse => "Ellipse",
            Self::Triangle => "Triangle",
            Self::Diamond => "Diamond",
            Self::Pentagon => "Pentagon",
            Self::Hexagon => "Hexagon",
            Self::Octagon => "Octagon",
            Self::Star => "Star",
            Self::Heart => "Heart",
            Self::RoundRect => "Rounded rectangle",
            Self::Parallelogram => "Parallelogram",
            Self::Trapezoid => "Trapezoid",
            Self::ArrowShape => "Arrow",
            Self::DoubleArrow => "Double arrow",
            Self::Cross => "Cross",
            Self::HomePlate => "Home plate",
            Self::Chevron => "Chevron",
            Self::Cylinder => "Cylinder",
            Self::Cloud => "Cloud",
            Self::Note => "Note",
            Self::Text => "Text",
            Self::Sticky => "Sticky note",
            Self::Image => "Image",
            Self::Frame => "Frame",
            Self::Draw => "Draw",
            Self::Highlighter => "Highlighter",
            Self::Line => "Line",
            Self::Polyline => "Polyline",
            Self::Arrow => "Arrow",
            Self::Laser => "Laser",
            Self::Group => "Group",
        }
    }

    #[must_use]
    pub const fn is_linear(self) -> bool {
        matches!(
            self,
            Self::Draw
                | Self::Highlighter
                | Self::Line
                | Self::Polyline
                | Self::Arrow
                | Self::Laser
        )
    }

    #[must_use]
    pub const fn is_textual(self) -> bool {
        matches!(self, Self::Text | Self::Sticky | Self::Note)
    }

    #[must_use]
    pub const fn is_geometric(self) -> bool {
        !matches!(
            self,
            Self::Text
                | Self::Sticky
                | Self::Image
                | Self::Frame
                | Self::Draw
                | Self::Highlighter
                | Self::Line
                | Self::Polyline
                | Self::Arrow
                | Self::Laser
                | Self::Group
        )
    }

    #[must_use]
    pub const fn is_container(self) -> bool {
        matches!(self, Self::Frame)
    }

    #[must_use]
    pub const fn has_text(self) -> bool {
        self.is_textual() || matches!(self, Self::Arrow | Self::Line | Self::Frame)
    }
}

/// How an arrow's path is routed between its endpoints.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum ArrowStyle {
    #[default]
    Straight,
    Bent,
    Elbow,
}

impl ArrowStyle {
    pub const ALL: [Self; 3] = [Self::Straight, Self::Bent, Self::Elbow];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Straight => "straight",
            Self::Bent => "bent",
            Self::Elbow => "elbow",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "straight" => Some(Self::Straight),
            "bent" | "curve" | "curved" => Some(Self::Bent),
            "elbow" => Some(Self::Elbow),
            _ => None,
        }
    }
}

/// Decoration drawn at an arrow endpoint.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum ArrowHead {
    #[default]
    None,
    Arrow,
    Triangle,
    Diamond,
    Square,
    Circle,
}

impl ArrowHead {
    pub const ALL: [Self; 6] = [
        Self::None,
        Self::Arrow,
        Self::Triangle,
        Self::Diamond,
        Self::Square,
        Self::Circle,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Arrow => "arrow",
            Self::Triangle => "triangle",
            Self::Diamond => "diamond",
            Self::Square => "square",
            Self::Circle => "circle",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "none" => Some(Self::None),
            "arrow" => Some(Self::Arrow),
            "triangle" => Some(Self::Triangle),
            "diamond" => Some(Self::Diamond),
            "square" => Some(Self::Square),
            "circle" | "dot" => Some(Self::Circle),
            _ => None,
        }
    }

    #[must_use]
    pub const fn is_visible(self) -> bool {
        !matches!(self, Self::None)
    }
}

/// A single drawable record.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShapeRecord {
    pub id: ShapeId,
    #[serde(default)]
    pub kind: ShapeKind,
    #[serde(default)]
    pub pos: Vec2,
    #[serde(default)]
    pub size: Vec2,
    /// Rotation in radians around the record's centre.
    #[serde(default)]
    pub rotation: f64,
    #[serde(default)]
    pub flip_x: bool,
    #[serde(default)]
    pub flip_y: bool,
    #[serde(default)]
    pub style: Style,
    #[serde(default)]
    pub text: String,
    /// Image source: data URL, http(s) URL, or absolute path.
    #[serde(default)]
    pub src: Option<String>,
    /// Absolute points for freehand, line, polyline, and arrow records.
    #[serde(default)]
    pub points: Vec<Vec2>,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub group_id: Option<GroupId>,
    #[serde(default)]
    pub parent_id: Option<ShapeId>,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub arrow_start: Option<ShapeId>,
    #[serde(default)]
    pub arrow_end: Option<ShapeId>,
    #[serde(default = "default_arrow_style")]
    pub arrow_style: String,
    #[serde(default = "default_start_head")]
    pub arrow_start_head: String,
    #[serde(default = "default_end_head")]
    pub arrow_end_head: String,
    #[serde(default)]
    pub z: i32,
    /// Unknown keys from newer documents, preserved round-trip.
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

fn default_arrow_style() -> String {
    "straight".into()
}

fn default_start_head() -> String {
    "none".into()
}

fn default_end_head() -> String {
    "arrow".into()
}

impl ShapeRecord {
    #[must_use]
    pub fn new(id: ShapeId, kind: ShapeKind, pos: Vec2, size: Vec2) -> Self {
        Self {
            id,
            kind,
            pos,
            size: Vec2::new(size.x.max(0.0), size.y.max(0.0)),
            rotation: 0.0,
            flip_x: false,
            flip_y: false,
            style: Style::default(),
            text: String::new(),
            src: None,
            points: Vec::new(),
            locked: false,
            group_id: None,
            parent_id: None,
            label: String::new(),
            arrow_start: None,
            arrow_end: None,
            arrow_style: default_arrow_style(),
            arrow_start_head: default_start_head(),
            arrow_end_head: default_end_head(),
            z: 0,
            extra: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    #[must_use]
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self
    }

    #[must_use]
    pub fn with_points(mut self, points: Vec<Vec2>) -> Self {
        self.set_points(points);
        self
    }

    /// Axis-aligned bounds before rotation.
    #[must_use]
    pub fn rect(&self) -> Rect {
        if self.points.is_empty() {
            Rect::new(self.pos, self.pos + self.size)
        } else {
            Rect::from_points(&self.points)
        }
    }

    #[must_use]
    pub fn center(&self) -> Vec2 {
        self.rect().center()
    }

    #[must_use]
    pub fn arrow_style(&self) -> ArrowStyle {
        ArrowStyle::parse(&self.arrow_style).unwrap_or_default()
    }

    #[must_use]
    pub fn start_head(&self) -> ArrowHead {
        ArrowHead::parse(&self.arrow_start_head).unwrap_or_default()
    }

    #[must_use]
    pub fn end_head(&self) -> ArrowHead {
        ArrowHead::parse(&self.arrow_end_head).unwrap_or_default()
    }

    #[must_use]
    pub fn display_text(&self) -> &str {
        if self.text.is_empty() {
            &self.label
        } else {
            &self.text
        }
    }

    /// Transform from local space (unrotated rectangle) to world space.
    #[must_use]
    pub fn world_transform(&self) -> Transform {
        let center = self.center();
        let flip = Transform::translate(center)
            * Transform::scale(Vec2::new(
                if self.flip_x { -1.0 } else { 1.0 },
                if self.flip_y { -1.0 } else { 1.0 },
            ))
            * Transform::translate(-center);
        Transform::rotation_about(self.rotation, center) * flip
    }

    #[must_use]
    pub fn world_bounds(&self) -> Rect {
        let transform = self.world_transform();
        let mut bounds = if self.points.is_empty() {
            transform.apply_rect(self.rect())
        } else {
            Rect::from_points(
                &self
                    .points
                    .iter()
                    .copied()
                    .map(|point| transform.apply(point))
                    .collect::<Vec<_>>(),
            )
        };
        bounds = bounds.union(transform.apply_rect(self.rect()));
        bounds
    }

    #[must_use]
    pub fn to_local(&self, point: Vec2) -> Vec2 {
        self.world_transform()
            .inverse()
            .map_or(point, |transform| transform.apply(point))
    }

    /// Hit test in world space, rotation-aware.
    #[must_use]
    pub fn hit_test(&self, point: Vec2, tolerance: f64) -> bool {
        let local = self.to_local(point);
        let rect = self.rect();
        if self.kind.is_linear() {
            if self.points.len() < 2 {
                return rect.expand(tolerance).contains(local);
            }
            let half_width = (self.style.stroke_width * 0.5).max(tolerance);
            return self.points.windows(2).any(|pair| {
                drawcraft_geometry::distance_point_segment(local, pair[0], pair[1]) <= half_width
            });
        }
        if self.kind == ShapeKind::Text && self.display_text().is_empty() {
            return rect.expand(tolerance).contains(local);
        }
        rect.expand(tolerance).contains(local)
    }

    pub fn apply_translation(&mut self, delta: Vec2) {
        self.pos += delta;
        for point in &mut self.points {
            *point += delta;
        }
    }

    pub fn scale_from(&mut self, anchor: Vec2, scale: Vec2) {
        let sx = if scale.x.is_finite() { scale.x } else { 1.0 };
        let sy = if scale.y.is_finite() { scale.y } else { 1.0 };
        self.pos = anchor + (self.pos - anchor) * Vec2::new(sx, sy);
        self.size = Vec2::new(
            (self.size.x * sx).abs().max(1.0),
            (self.size.y * sy).abs().max(1.0),
        );
        for point in &mut self.points {
            *point = anchor + (*point - anchor) * Vec2::new(sx, sy);
        }
    }

    pub fn rotate_by(&mut self, radians: f64) {
        if radians.is_finite() {
            self.rotation = (self.rotation + radians) % std::f64::consts::TAU;
        }
    }

    pub fn flip_axis(&mut self, horizontal: bool) {
        if horizontal {
            self.flip_x = !self.flip_x;
        } else {
            self.flip_y = !self.flip_y;
        }
    }

    pub fn set_points(&mut self, mut points: Vec<Vec2>) {
        if points.len() > crate::MAX_POINTS_PER_SHAPE {
            points.truncate(crate::MAX_POINTS_PER_SHAPE);
        }
        let bounds = Rect::from_points(&points);
        if !bounds.is_empty() {
            self.pos = bounds.min;
            self.size = bounds.size();
        }
        self.points = points;
    }

    /// Point on the shape's rectangle edge in the direction of `toward`.
    #[must_use]
    pub fn anchor_toward(&self, toward: Vec2) -> Vec2 {
        rect_anchor(self.rect(), toward)
    }

    /// Recompute `pos`/`size` from points after direct point edits.
    pub fn sync_rect_from_points(&mut self) {
        if self.points.is_empty() {
            return;
        }
        let bounds = Rect::from_points(&self.points);
        if !bounds.is_empty() {
            self.pos = bounds.min;
            self.size = bounds.size();
        }
    }

    /// Whether the record carries an arrow binding.
    #[must_use]
    pub fn is_bound_arrow(&self) -> bool {
        self.kind == ShapeKind::Arrow && (self.arrow_start.is_some() || self.arrow_end.is_some())
    }

    /// Interior endpoints of an arrow. Missing points fall back to the rect.
    #[must_use]
    pub fn endpoints(&self) -> (Vec2, Vec2) {
        let rect = self.rect();
        let start = self.points.first().copied().unwrap_or(rect.min);
        let end = self.points.last().copied().unwrap_or(rect.max);
        (start, end)
    }

    /// Validate shape-level invariants. Style is validated separately.
    pub fn validate(&self) -> std::result::Result<(), String> {
        if self.id.is_empty() {
            return Err("id must not be empty".into());
        }
        if !self.pos.is_finite() || !self.size.is_finite() || !self.rotation.is_finite() {
            return Err("position, size, and rotation must be finite".into());
        }
        if self.size.x < 0.0 || self.size.y < 0.0 {
            return Err("size must not be negative".into());
        }
        if self.points.len() > crate::MAX_POINTS_PER_SHAPE {
            return Err(format!(
                "{} points exceeds the {} limit",
                self.points.len(),
                crate::MAX_POINTS_PER_SHAPE
            ));
        }
        if self.points.iter().any(|point| !point.is_finite()) {
            return Err("points must be finite".into());
        }
        self.style.validate()?;
        Ok(())
    }

    /// Width used by the dashed-stroke renderer.
    #[must_use]
    pub fn effective_stroke_width(&self) -> f64 {
        self.style.stroke_width.max(0.0)
    }

    #[must_use]
    pub fn dash(&self) -> Dash {
        self.style.dash
    }
}

/// Point on the edge of `rect` in the direction of `toward` from its centre.
#[must_use]
pub fn rect_anchor(rect: Rect, toward: Vec2) -> Vec2 {
    let center = rect.center();
    let direction = toward - center;
    if direction.length_squared() <= f64::EPSILON || rect.is_empty() {
        return center;
    }
    let half = rect.size() * 0.5;
    let scale_x = if direction.x.abs() > f64::EPSILON {
        half.x / direction.x.abs()
    } else {
        f64::INFINITY
    };
    let scale_y = if direction.y.abs() > f64::EPSILON {
        half.y / direction.y.abs()
    } else {
        f64::INFINITY
    };
    let scale = scale_x.min(scale_y);
    if scale.is_finite() {
        center + direction * scale
    } else {
        center
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ShapeRecord {
        ShapeRecord::new(
            ShapeId::from("shape-1"),
            ShapeKind::Rectangle,
            Vec2::new(0.0, 0.0),
            Vec2::new(100.0, 50.0),
        )
    }

    #[test]
    fn bounds_include_points() {
        let shape = sample().with_points(vec![Vec2::new(0.0, 0.0), Vec2::new(200.0, 100.0)]);
        assert_eq!(shape.rect(), Rect::from_xywh(0.0, 0.0, 200.0, 100.0));
    }

    #[test]
    fn rotation_widens_world_bounds() {
        let mut shape = sample();
        shape.rotation = std::f64::consts::FRAC_PI_4;
        assert!(shape.world_bounds().width() > 100.0);
    }

    #[test]
    fn hit_test_respects_rotation() {
        let mut shape = sample();
        shape.rotation = std::f64::consts::FRAC_PI_2;
        // A point outside the unrotated rectangle but inside the rotated one.
        assert!(shape.hit_test(Vec2::new(50.0, -20.0), 0.0));
    }

    #[test]
    fn arrow_style_parses() {
        assert_eq!(ArrowStyle::parse("elbow"), Some(ArrowStyle::Elbow));
        assert_eq!(ShapeKind::parse("round_rect"), Some(ShapeKind::RoundRect));
        assert_eq!(ShapeKind::GEOMETRIC.len(), 20);
        assert_eq!(ShapeKind::ALL.len(), 31);
    }

    #[test]
    fn validates_finite_geometry() {
        let mut shape = sample();
        shape.pos = Vec2::new(f64::NAN, 0.0);
        assert!(shape.validate().is_err());
    }
}
