//! Native `.drawcraft` format and exporters.
//!
//! The native format is documented in `FORMAT.md`: UTF-8 JSON, versioned, with
//! unknown fields preserved by the document model. Export is deliberately
//! shared with the on-screen renderer via `drawcraft_render::Scene`.

#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::field_reassign_with_default
    )
)]

use std::path::Path;

use drawcraft_document::{Document, ShapeId};
use drawcraft_render::{RenderError, SvgOptions, document_scene, render_png, scene_svg};
use thiserror::Error;

/// Maximum accepted document size. Guards against accidentally loading a huge
/// file into memory.
pub const MAX_BYTES: u64 = 16 * 1024 * 1024;
/// Soft warning threshold for exported SVG/PNG dimensions.
pub const MAX_EXPORT_SCALE: f64 = 8.0;

/// Format, export, and I/O errors.
#[derive(Debug, Error)]
pub enum FormatError {
    #[error("document is too large ({0} bytes, limit {MAX_BYTES})")]
    TooLarge(u64),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid document: {0}")]
    Invalid(String),
    #[error("document validation failed: {0}")]
    Document(#[from] drawcraft_document::DocumentError),
    #[error("render error: {0}")]
    Render(#[from] RenderError),
    #[error("nothing is selected")]
    EmptySelection,
    #[error("unsupported export scale {0}")]
    InvalidScale(f64),
}

pub type Result<T> = std::result::Result<T, FormatError>;

/// What part of a document to export.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum ExportScope {
    #[default]
    Page,
    Selection,
}

impl ExportScope {
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "page" => Some(Self::Page),
            "selection" | "selected" => Some(Self::Selection),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Page => "page",
            Self::Selection => "selection",
        }
    }
}

/// Serialise a document to pretty JSON with a trailing newline.
pub fn to_json(document: &Document) -> Result<String> {
    document.validate()?;
    let mut json = serde_json::to_string_pretty(document)?;
    json.push('\n');
    Ok(json)
}

/// Parse and validate a document.
pub fn from_json(text: &str) -> Result<Document> {
    if text.len() as u64 > MAX_BYTES {
        return Err(FormatError::TooLarge(text.len() as u64));
    }
    if text.trim().is_empty() {
        return Err(FormatError::Invalid("document is empty".into()));
    }
    let document: Document = serde_json::from_str(text)?;
    document.validate()?;
    Ok(document)
}

/// Load a document from disk, enforcing [`MAX_BYTES`].
pub fn load(path: impl AsRef<Path>) -> Result<Document> {
    let path = path.as_ref();
    let metadata = std::fs::metadata(path)?;
    if metadata.len() > MAX_BYTES {
        return Err(FormatError::TooLarge(metadata.len()));
    }
    let bytes = std::fs::read(path)?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(FormatError::TooLarge(bytes.len() as u64));
    }
    let text = String::from_utf8(bytes)
        .map_err(|_| FormatError::Invalid("document is not valid UTF-8".into()))?;
    from_json(&text)
}

/// Save a document to disk atomically (write temp file, then rename).
pub fn save(document: &Document, path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    let json = to_json(document)?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    if !parent.as_os_str().is_empty() {
        std::fs::create_dir_all(parent)?;
    }
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("document.drawcraft");
    let temporary = parent.join(format!(".{file_name}.tmp"));
    std::fs::write(&temporary, json.as_bytes())?;
    if let Err(error) = std::fs::rename(&temporary, path) {
        let _ = std::fs::remove_file(&temporary);
        return Err(FormatError::Io(error));
    }
    Ok(())
}

/// Build the scene that would be exported.
#[must_use]
pub fn export_scene(
    document: &Document,
    scope: ExportScope,
    selection: &[ShapeId],
) -> drawcraft_render::Scene {
    match scope {
        ExportScope::Page => document_scene(document, None),
        ExportScope::Selection => document_scene(document, Some(selection)),
    }
}

/// Render an SVG string.
pub fn export_svg_string(
    document: &Document,
    scope: ExportScope,
    selection: &[ShapeId],
    transparent: bool,
) -> Result<String> {
    if scope == ExportScope::Selection && selection.is_empty() {
        return Err(FormatError::EmptySelection);
    }
    let scene = export_scene(document, scope, selection);
    let options = SvgOptions {
        padding: 24.0,
        background: (!transparent).then(|| "#ffffff".to_owned()),
    };
    Ok(scene_svg(&scene, &options))
}

/// Render PNG bytes.
pub fn export_png_bytes(
    document: &Document,
    scope: ExportScope,
    selection: &[ShapeId],
    scale: f64,
    transparent: bool,
) -> Result<Vec<u8>> {
    if scope == ExportScope::Selection && selection.is_empty() {
        return Err(FormatError::EmptySelection);
    }
    if !scale.is_finite() || scale <= 0.0 || scale > MAX_EXPORT_SCALE {
        return Err(FormatError::InvalidScale(scale));
    }
    let scene = export_scene(document, scope, selection);
    let options = SvgOptions {
        padding: 24.0,
        background: (!transparent).then(|| "#ffffff".to_owned()),
    };
    let svg = scene_svg(&scene, &options);
    render_png(&svg, scale, transparent).map_err(FormatError::Render)
}

/// Write an SVG export.
pub fn export_svg(
    document: &Document,
    scope: ExportScope,
    selection: &[ShapeId],
    path: impl AsRef<Path>,
    transparent: bool,
) -> Result<()> {
    let svg = export_svg_string(document, scope, selection, transparent)?;
    std::fs::write(path, svg)?;
    Ok(())
}

/// Write a PNG export.
pub fn export_png(
    document: &Document,
    scope: ExportScope,
    selection: &[ShapeId],
    path: impl AsRef<Path>,
    scale: f64,
    transparent: bool,
) -> Result<()> {
    let bytes = export_png_bytes(document, scope, selection, scale, transparent)?;
    std::fs::write(path, bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use drawcraft_document::{ShapeKind, ShapeRecord, Vec2};
    use serde_json::json;

    fn sample() -> Document {
        let mut document = Document::new();
        let shape = ShapeRecord::new(
            ShapeId::from("shape-1"),
            ShapeKind::Rectangle,
            Vec2::new(10.0, 20.0),
            Vec2::new(100.0, 50.0),
        );
        let _ = document.add_shape(shape);
        document
    }

    #[test]
    fn json_round_trip() {
        let document = sample();
        let json = to_json(&document).unwrap_or_default();
        let loaded = from_json(&json).unwrap_or_default();
        assert_eq!(loaded, document);
    }

    #[test]
    fn unknown_fields_are_preserved() {
        let raw = json!({
            "version": 1,
            "active_page": "page-1",
            "pages": [{
                "id": "page-1",
                "name": "Page 1",
                "shapes": [],
                "future_page_field": {"kept": true}
            }],
            "meta": {"author": "agent"},
            "future_top_level": [1, 2, 3]
        })
        .to_string();
        let document = from_json(&raw).unwrap_or_default();
        let saved = to_json(&document).unwrap_or_default();
        assert!(saved.contains("future_top_level"));
        assert!(saved.contains("future_page_field"));
    }

    #[test]
    fn rejects_oversized_documents() {
        let huge = " ".repeat(MAX_BYTES as usize + 1);
        assert!(matches!(from_json(&huge), Err(FormatError::TooLarge(_))));
    }

    #[test]
    fn saves_and_loads_atomically() {
        let directory = tempfile::tempdir().unwrap_or_else(|_| panic!("temp dir"));
        let path = directory.path().join("test.drawcraft");
        let document = sample();
        save(&document, &path).unwrap_or_else(|error| panic!("{error}"));
        let loaded = load(&path).unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(loaded, document);
        assert!(path.exists());
    }

    #[test]
    fn exports_svg_and_png() {
        let document = sample();
        let svg = export_svg_string(&document, ExportScope::Page, &[], false)
            .unwrap_or_else(|error| panic!("{error}"));
        assert!(svg.contains("<svg"));
        assert!(svg.contains("<path"));

        let png = export_png_bytes(&document, ExportScope::Page, &[], 1.0, false)
            .unwrap_or_else(|error| panic!("{error}"));
        assert!(png.starts_with(&[0x89, b'P', b'N', b'G']));
    }

    #[test]
    fn selection_export_requires_selection() {
        let document = sample();
        assert!(matches!(
            export_svg_string(&document, ExportScope::Selection, &[], false),
            Err(FormatError::EmptySelection)
        ));
    }
}
