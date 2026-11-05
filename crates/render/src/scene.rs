use std::collections::BTreeMap;

use drawcraft_document::{
    ArrowHead, Dash, Fill, GroupId, Page, Rect, ShapeId, ShapeKind, ShapeRecord, Vec2,
};

use crate::path::Path;
use crate::shapes;
use crate::text::{TextAlign, line_height, text_width, wrap_text};

/// A filled path in the scene.
#[derive(Clone, Debug, PartialEq)]
pub struct FillPaint {
    pub path: Path,
    pub color: String,
    pub opacity: f64,
    pub pattern: bool,
}

/// A stroked path in the scene.
#[derive(Clone, Debug, PartialEq)]
pub struct StrokePaint {
    pub path: Path,
    pub color: String,
    pub width: f64,
    pub dash: Dash,
    pub opacity: f64,
}

/// Text laid out inside a box.
#[derive(Clone, Debug, PartialEq)]
pub struct TextItem {
    pub text: String,
    pub pos: Vec2,
    pub width: f64,
    pub font_size: f64,
    pub font_family: String,
    pub color: String,
    pub opacity: f64,
    pub align: TextAlign,
    pub rotation: f64,
    pub rotation_center: Vec2,
}

impl TextItem {
    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        wrap_text(&self.text, self.width, self.font_size)
    }

    #[must_use]
    pub fn height(&self) -> f64 {
        self.lines().len() as f64 * line_height(self.font_size)
    }

    #[must_use]
    pub fn anchor_x(&self) -> f64 {
        match self.align {
            TextAlign::Left => self.pos.x,
            TextAlign::Center => self.pos.x + self.width * 0.5,
            TextAlign::Right => self.pos.x + self.width,
        }
    }

    #[must_use]
    pub fn estimated_width(&self) -> f64 {
        self.lines()
            .iter()
            .map(|line| text_width(line, self.font_size))
            .fold(0.0, f64::max)
            .min(self.width)
    }
}

/// An embedded or referenced bitmap.
#[derive(Clone, Debug, PartialEq)]
pub struct ImageItem {
    pub href: String,
    pub rect: Rect,
}

/// One z-ordered drawable.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneItem {
    pub id: Option<ShapeId>,
    pub kind: ShapeKind,
    pub z: i32,
    pub fills: Vec<FillPaint>,
    pub strokes: Vec<StrokePaint>,
    /// Diagonal guide lines for pattern fills; the SVG writer uses a `<pattern>`
    /// definition instead, while the GPU UI draws these directly.
    pub hatch: Vec<Path>,
    pub text: Option<TextItem>,
    pub image: Option<ImageItem>,
    pub rotation: f64,
    pub rotation_center: Vec2,
    pub opacity: f64,
    pub locked: bool,
    pub clip: Option<Rect>,
    pub group: Option<GroupId>,
}

impl SceneItem {
    fn new(shape: &ShapeRecord) -> Self {
        Self {
            id: Some(shape.id.clone()),
            kind: shape.kind,
            z: shape.z,
            fills: Vec::new(),
            strokes: Vec::new(),
            hatch: Vec::new(),
            text: None,
            image: None,
            rotation: shape.rotation,
            rotation_center: shape.center(),
            opacity: shape.style.opacity,
            locked: shape.locked,
            clip: None,
            group: shape.group_id.clone(),
        }
    }

    #[must_use]
    pub fn bounds(&self) -> Rect {
        let mut bounds = Rect::EMPTY;
        for fill in &self.fills {
            bounds = bounds.union(fill.path.bounds());
        }
        for stroke in &self.strokes {
            bounds = bounds.union(stroke.path.bounds().expand(stroke.width * 0.5 + 1.0));
        }
        for path in &self.hatch {
            bounds = bounds.union(path.bounds());
        }
        if let Some(text) = &self.text {
            let height = text.lines().len() as f64 * line_height(text.font_size);
            bounds = bounds.union(Rect::from_xywh(
                text.pos.x,
                text.pos.y,
                text.width.max(text.estimated_width()),
                height,
            ));
        }
        if let Some(image) = &self.image {
            bounds = bounds.union(image.rect);
        }
        bounds
    }
}

/// A fully resolved page ready for rendering.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Scene {
    pub items: Vec<SceneItem>,
    pub bounds: Rect,
}

impl Scene {
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }
}

/// Build a scene for one page, optionally filtered to selected shapes.
#[must_use]
pub fn build_scene(page: &Page, selection: Option<&[ShapeId]>) -> Scene {
    let frame_clips: BTreeMap<ShapeId, Rect> = page
        .shapes
        .iter()
        .filter(|shape| shape.kind == ShapeKind::Frame)
        .map(|shape| (shape.id.clone(), shape.world_bounds()))
        .collect();

    let mut items = Vec::new();
    for shape in page.shapes_sorted_by_z() {
        if let Some(selection) = selection
            && !selection.iter().any(|id| id == &shape.id)
        {
            continue;
        }
        if shape.kind == ShapeKind::Group {
            continue;
        }
        let mut item = SceneItem::new(shape);
        item.clip = shape
            .parent_id
            .as_ref()
            .and_then(|parent| frame_clips.get(parent))
            .copied();
        let transform = shape.world_transform();
        if shape.kind.is_linear() {
            build_linear(&mut item, shape, transform);
        } else {
            build_closed(&mut item, shape, transform);
        }
        items.push(item);
    }

    let bounds = items
        .iter()
        .map(SceneItem::bounds)
        .reduce(Rect::union)
        .unwrap_or_else(|| Rect::from_xywh(0.0, 0.0, 800.0, 600.0));

    Scene { items, bounds }
}

fn build_closed(
    item: &mut SceneItem,
    shape: &ShapeRecord,
    transform: drawcraft_geometry::Transform,
) {
    let rect = shape.rect();
    let style = &shape.style;
    match shape.kind {
        ShapeKind::Text => {
            item.text = Some(text_box(
                shape,
                Vec2::new(rect.min.x + 4.0, rect.min.y + 2.0),
                rect.width().max(24.0),
                TextAlign::Left,
            ));
            return;
        }
        ShapeKind::Image => {
            if let Some(src) = &shape.src {
                item.image = Some(ImageItem {
                    href: src.clone(),
                    rect,
                });
            }
            if style.dash != Dash::None && style.stroke_width > 0.0 {
                let path = shapes::shape_path(ShapeKind::Rectangle, rect).transformed(transform);
                item.strokes.push(StrokePaint {
                    path,
                    color: style.stroke.clone(),
                    width: style.stroke_width,
                    dash: style.dash,
                    opacity: 1.0,
                });
            }
            return;
        }
        _ => {}
    }

    let path = shapes::shape_path(shape.kind, rect).transformed(transform);
    if style.fill != Fill::None {
        item.fills.push(FillPaint {
            path: path.clone(),
            color: style.fill_color.clone(),
            opacity: style.fill.alpha(),
            pattern: style.fill == Fill::Pattern,
        });
        if style.fill == Fill::Pattern {
            let spacing = (style.stroke_width * 3.0).clamp(4.0, 16.0);
            item.hatch = shapes::pattern_lines(rect, spacing)
                .into_iter()
                .map(|line| line.transformed(transform))
                .collect();
        }
    }

    if style.dash != Dash::None && style.stroke_width > 0.0 {
        let stroke_path = if style.dash == Dash::HandDrawn {
            shapes::hand_drawn_path(&path, seed_for(shape), 1.1)
        } else {
            path
        };
        item.strokes.push(StrokePaint {
            path: stroke_path,
            color: style.stroke.clone(),
            width: style.stroke_width,
            dash: style.dash,
            opacity: 1.0,
        });
    }

    if matches!(
        shape.kind,
        ShapeKind::Sticky | ShapeKind::Note | ShapeKind::Frame
    ) {
        let (position, width, align) = match shape.kind {
            ShapeKind::Frame => (
                Vec2::new(rect.min.x + 8.0, rect.min.y + 8.0),
                (rect.width() - 16.0).max(24.0),
                TextAlign::Left,
            ),
            _ => (
                Vec2::new(rect.min.x + 16.0, rect.min.y + 16.0),
                (rect.width() - 32.0).max(24.0),
                TextAlign::Left,
            ),
        };
        if !shape.display_text().is_empty() {
            item.text = Some(text_box(shape, position, width, align));
        }
    }
}

fn build_linear(
    item: &mut SceneItem,
    shape: &ShapeRecord,
    transform: drawcraft_geometry::Transform,
) {
    let style = &shape.style;
    let fallback = shape.rect();
    let path = shapes::linear_path(&shape.points, fallback).transformed(transform);
    let stroke_path = if style.dash == Dash::HandDrawn {
        shapes::hand_drawn_path(&path, seed_for(shape), 0.9)
    } else {
        path.clone()
    };
    if style.stroke_width > 0.0 && style.dash != Dash::None {
        item.strokes.push(StrokePaint {
            path: stroke_path,
            color: style.stroke.clone(),
            width: style.stroke_width,
            dash: style.dash,
            opacity: 1.0,
        });
    }

    if shape.kind == ShapeKind::Arrow {
        let points = &shape.points;
        if points.len() >= 2 {
            let size = (style.stroke_width * 3.5).max(10.0);
            let end = points[points.len() - 1];
            let end_from = points[points.len() - 2];
            let start = points[0];
            let start_from = points[1];
            let mut add_head = |head: ArrowHead, tip: Vec2, from: Vec2| {
                for head_path in shapes::arrow_head_paths(head, tip, from, size) {
                    let head_path = head_path.transformed(transform);
                    if head == ArrowHead::Arrow {
                        item.strokes.push(StrokePaint {
                            path: head_path,
                            color: style.stroke.clone(),
                            width: style.stroke_width.max(1.0),
                            dash: Dash::Solid,
                            opacity: 1.0,
                        });
                    } else {
                        item.fills.push(FillPaint {
                            path: head_path,
                            color: style.stroke.clone(),
                            opacity: 1.0,
                            pattern: false,
                        });
                    }
                }
            };
            add_head(shape.end_head(), end, end_from);
            add_head(shape.start_head(), start, start_from);
        }
        if !shape.display_text().is_empty()
            && let (Some(first), Some(last)) = (shape.points.first(), shape.points.last())
        {
            let mid = (*first + *last) * 0.5;
            item.text = Some(text_box(
                shape,
                Vec2::new(mid.x - 100.0, mid.y - line_height(style.font_size)),
                200.0,
                TextAlign::Center,
            ));
        }
    }
}

fn text_box(shape: &ShapeRecord, pos: Vec2, width: f64, align: TextAlign) -> TextItem {
    TextItem {
        text: shape.display_text().to_owned(),
        pos,
        width,
        font_size: shape.style.font_size,
        font_family: shape.style.font_family.clone(),
        color: shape.style.stroke.clone(),
        opacity: 1.0,
        align,
        rotation: shape.rotation,
        rotation_center: shape.center(),
    }
}

fn seed_for(shape: &ShapeRecord) -> f64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in shape.id.as_str().bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    (hash % 1000) as f64 * 0.017
}

#[cfg(test)]
mod tests {
    use super::*;
    use drawcraft_document::{Document, ShapeKind, ShapeRecord};

    #[test]
    fn builds_scene_for_rectangle() {
        let mut document = Document::new();
        let shape = ShapeRecord::new(
            ShapeId::from("shape-1"),
            ShapeKind::Rectangle,
            Vec2::ZERO,
            Vec2::new(100.0, 50.0),
        );
        let _ = document.add_shape(shape);
        let fallback = Page::new("p".into(), "p");
        let scene = build_scene(document.active().unwrap_or(&fallback), None);
        assert_eq!(scene.items.len(), 1);
        assert!(!scene.items[0].fills.is_empty());
        assert!(!scene.items[0].strokes.is_empty());
        assert!(scene.bounds.width() >= 100.0);
    }

    #[test]
    fn selection_filter_limits_items() {
        let mut document = Document::new();
        for index in 0..3 {
            let shape = ShapeRecord::new(
                ShapeId::generated(index + 1),
                ShapeKind::Rectangle,
                Vec2::new(index as f64 * 50.0, 0.0),
                Vec2::new(20.0, 20.0),
            );
            let _ = document.add_shape(shape);
        }
        let fallback = Page::new("p".into(), "p");
        let page = document.active().unwrap_or(&fallback);
        let scene = build_scene(page, Some(&[ShapeId::generated(2)]));
        assert_eq!(scene.items.len(), 1);
    }

    #[test]
    fn solid_line_emits_a_stroke() {
        let mut document = Document::new();
        let shape = ShapeRecord::new(
            ShapeId::from("line-1"),
            ShapeKind::Line,
            Vec2::new(0.0, 0.0),
            Vec2::new(100.0, 50.0),
        )
        .with_points(vec![Vec2::new(0.0, 0.0), Vec2::new(100.0, 50.0)]);
        let _ = document.add_shape(shape);
        let fallback = Page::new("p".into(), "p");
        let scene = build_scene(document.active().unwrap_or(&fallback), None);
        assert_eq!(scene.items.len(), 1);
        assert!(
            !scene.items[0].strokes.is_empty(),
            "a solid line must emit a stroke"
        );
    }

    #[test]
    fn polyline_emits_stroke_through_all_points() {
        let mut document = Document::new();
        let shape = ShapeRecord::new(
            ShapeId::from("poly-1"),
            ShapeKind::Polyline,
            Vec2::ZERO,
            Vec2::new(100.0, 50.0),
        )
        .with_points(vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(50.0, 50.0),
            Vec2::new(100.0, 0.0),
        ]);
        let _ = document.add_shape(shape);
        let fallback = Page::new("p".into(), "p");
        let scene = build_scene(document.active().unwrap_or(&fallback), None);
        assert_eq!(scene.items.len(), 1);
        assert!(!scene.items[0].strokes.is_empty());
        let points = &scene.items[0].strokes[0].path.flatten(16);
        assert!(
            points.len() >= 3,
            "a polyline stroke must pass through all its vertices"
        );
    }
}
