use thiserror::Error;

use crate::{PageId, ShapeId};

/// Errors produced when validating a document or applying a document edit.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DocumentError {
    #[error("unsupported document version {0}")]
    UnsupportedVersion(u32),
    #[error("document must contain at least one page")]
    NoPages,
    #[error("too many pages: {actual} (limit {limit})")]
    TooManyPages { actual: usize, limit: usize },
    #[error("too many shapes on page {page}: {actual} (limit {limit})")]
    TooManyShapes {
        page: PageId,
        actual: usize,
        limit: usize,
    },
    #[error("too many points on shape {shape}: {actual} (limit {limit})")]
    TooManyPoints {
        shape: ShapeId,
        actual: usize,
        limit: usize,
    },
    #[error("duplicate page id: {0}")]
    DuplicatePage(PageId),
    #[error("duplicate shape id: {0}")]
    DuplicateShape(ShapeId),
    #[error("active page not found: {0}")]
    MissingActivePage(PageId),
    #[error("invalid shape {id}: {reason}")]
    InvalidShape { id: ShapeId, reason: String },
    #[error("invalid page {id}: {reason}")]
    InvalidPage { id: PageId, reason: String },
    #[error("invalid document: {0}")]
    Invalid(String),
}
