//! DrawCraft document model.
//!
//! The model is deliberately plain data: cloning a [`Document`] is cheap enough
//! to serve as an undo snapshot, and every shape is a self-contained record with
//! an ID. Unknown fields are preserved through `extra` maps so that documents
//! written by newer builds can be read and re-saved without losing data.

#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::field_reassign_with_default
    )
)]

mod color;
mod document;
mod error;
mod ids;
mod shape;
mod style;

pub use color::{Color, parse_color};
pub use document::{Document, MAX_PAGES, MAX_POINTS_PER_SHAPE, MAX_SHAPES_PER_PAGE, Page};
pub use error::DocumentError;
pub use ids::{GroupId, PageId, ShapeId};
pub use shape::{ArrowHead, ArrowStyle, ShapeKind, ShapeRecord, rect_anchor};
pub use style::{Dash, Fill, Style};

pub use drawcraft_geometry::{Rect, Transform, Vec2};

/// Current native document format version.
pub const FORMAT_VERSION: u32 = 1;

pub type Result<T> = std::result::Result<T, DocumentError>;
