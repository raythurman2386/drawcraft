//! DrawCraft rendering primitives.
//!
//! The renderer is split into a resolution-independent scene (paths, text,
//! images), an SVG writer, and a PNG rasteriser. The egui front-end consumes
//! the same [`Scene`] so screen and export stay visually aligned.

#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::field_reassign_with_default
    )
)]

mod error;
pub mod mesh;
pub mod path;
mod png;
mod scene;
pub mod shapes;
pub mod svg;
pub mod text;

pub use error::RenderError;
pub use mesh::triangulate;
pub use path::{Path, Verb};
pub use png::{MAX_DIMENSION, MAX_PIXELS, render_png};
pub use scene::{FillPaint, ImageItem, Scene, SceneItem, StrokePaint, TextItem, build_scene};
pub use svg::{SvgOptions, scene_to_svg};
pub use text::{TextAlign, escape_xml, line_height, text_width, wrap_text};

use drawcraft_document::{Document, Page, ShapeId};
use drawcraft_geometry::Rect;

/// Build a scene for the active page.
#[must_use]
pub fn document_scene(document: &Document, selection: Option<&[ShapeId]>) -> Scene {
    document
        .active()
        .map_or_else(Scene::empty, |page| build_scene(page, selection))
}

/// Build a scene for a page.
#[must_use]
pub fn page_scene(page: &Page, selection: Option<&[ShapeId]>) -> Scene {
    build_scene(page, selection)
}

/// Render a scene to SVG.
#[must_use]
pub fn scene_svg(scene: &Scene, options: &SvgOptions) -> String {
    scene_to_svg(scene, options)
}

/// Render a scene directly to PNG bytes.
pub fn scene_png(scene: &Scene, options: &SvgOptions, scale: f64) -> Result<Vec<u8>, RenderError> {
    let svg = scene_to_svg(scene, options);
    render_png(&svg, scale, options.background.is_none())
}

/// Bounds used when a scene has no content.
#[must_use]
pub fn default_bounds() -> Rect {
    Rect::from_xywh(0.0, 0.0, 800.0, 600.0)
}
