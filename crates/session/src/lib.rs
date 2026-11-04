//! Workspace session: the bridge between the headless editor and files.
//!
//! `Session` owns an [`Editor`] plus the current path, dirty flag, recent-file
//! list, and export helpers. The UI, CLI, JSON channel, and MCP server all use
//! the same [`Session::execute`] entry point.

#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::field_reassign_with_default
    )
)]

use std::path::{Path, PathBuf};

use drawcraft_command::ids;
use drawcraft_editor::{EditError, Editor, Params};
use drawcraft_format::{ExportScope, FormatError};
use serde_json::{Value, json};
use thiserror::Error;

const MAX_RECENT: usize = 12;

/// Session-level failures.
#[derive(Debug, Error)]
pub enum SessionError {
    #[error("editor error: {0}")]
    Editor(#[from] EditError),
    #[error("format error: {0}")]
    Format(#[from] FormatError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("no document path; use save_as first")]
    NoPath,
    #[error("invalid request: {0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, SessionError>;

/// A document open in the workspace.
#[derive(Debug)]
pub struct Session {
    pub editor: Editor,
    pub path: Option<PathBuf>,
    pub dirty: bool,
    recent: Vec<PathBuf>,
}

impl Session {
    #[must_use]
    pub fn new() -> Self {
        Self {
            editor: Editor::new(),
            path: None,
            dirty: false,
            recent: load_recent(),
        }
    }

    #[must_use]
    pub fn recent(&self) -> &[PathBuf] {
        &self.recent
    }

    #[must_use]
    pub fn current_path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    #[must_use]
    pub fn display_name(&self) -> String {
        self.path
            .as_deref()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .unwrap_or("Untitled")
            .to_owned()
    }

    #[must_use]
    pub fn selection_ids(&self) -> Vec<drawcraft_document::ShapeId> {
        self.editor.selection.ids().to_vec()
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Reset to a blank document.
    pub fn new_document(&mut self) {
        self.editor.reset();
        self.path = None;
        self.dirty = false;
    }

    /// Open a document and make it current.
    pub fn open(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        let document = drawcraft_format::load(path)?;
        self.editor.replace_document(document)?;
        self.path = Some(path.to_path_buf());
        self.dirty = false;
        self.record_recent(path);
        Ok(())
    }

    /// Save to the current path.
    pub fn save(&mut self) -> Result<PathBuf> {
        let path = self.path.clone().ok_or(SessionError::NoPath)?;
        self.save_to(&path)?;
        Ok(path)
    }

    /// Save to an explicit path and make it current.
    pub fn save_as(&mut self, path: impl AsRef<Path>) -> Result<PathBuf> {
        let path = path.as_ref().to_path_buf();
        self.save_to(&path)?;
        self.path = Some(path.clone());
        self.record_recent(&path);
        Ok(path)
    }

    fn save_to(&mut self, path: &Path) -> Result<()> {
        drawcraft_format::save(&self.editor.doc, path)?;
        self.dirty = false;
        Ok(())
    }

    /// Open the last saved document path.
    pub fn reload(&mut self) -> Result<()> {
        let path = self.path.clone().ok_or(SessionError::NoPath)?;
        self.open(path)
    }

    /// Export the active page or selection as SVG.
    pub fn export_svg(
        &self,
        path: impl AsRef<Path>,
        scope: ExportScope,
        transparent: bool,
    ) -> Result<()> {
        drawcraft_format::export_svg(
            &self.editor.doc,
            scope,
            &self.selection_ids(),
            path,
            transparent,
        )?;
        Ok(())
    }

    /// Export the active page or selection as PNG.
    pub fn export_png(
        &self,
        path: impl AsRef<Path>,
        scope: ExportScope,
        scale: f64,
        transparent: bool,
    ) -> Result<()> {
        drawcraft_format::export_png(
            &self.editor.doc,
            scope,
            &self.selection_ids(),
            path,
            scale,
            transparent,
        )?;
        Ok(())
    }

    /// Execute a command ID. File/export commands are handled here; everything
    /// else delegates to the editor.
    pub fn execute(&mut self, id: &str, params: &Value) -> Result<Value> {
        let raw = params.clone();
        let params = Params::new(params)?;
        match id {
            ids::FILE_NEW => {
                self.new_document();
                Ok(json!({"ok": true}))
            }
            ids::FILE_OPEN => {
                let path = required_path(&params)?;
                self.open(&path)?;
                Ok(self.info())
            }
            ids::FILE_SAVE => {
                if let Some(path) = params.optional_string("path") {
                    let path = self.save_as(path)?;
                    Ok(json!({"path": path}))
                } else {
                    let path = self.save()?;
                    Ok(json!({"path": path}))
                }
            }
            ids::FILE_SAVE_AS => {
                let path = required_path(&params)?;
                let path = self.save_as(path)?;
                Ok(json!({"path": path}))
            }
            ids::FILE_EXPORT_SVG => {
                let path = required_path(&params)?;
                let scope = parse_scope(&params)?;
                let transparent = params.boolean_or("transparent", false);
                self.export_svg(&path, scope, transparent)?;
                Ok(json!({"path": path, "scope": scope.as_str()}))
            }
            ids::FILE_EXPORT_PNG => {
                let path = required_path(&params)?;
                let scope = parse_scope(&params)?;
                let scale = params.number_or("scale", 1.0);
                let transparent = params.boolean_or("transparent", false);
                self.export_png(&path, scope, scale, transparent)?;
                Ok(json!({"path": path, "scope": scope.as_str(), "scale": scale}))
            }
            ids::DOC_RECENT => Ok(json!({
                "recent": self.recent.iter().map(|path| path.display().to_string()).collect::<Vec<_>>()
            })),
            ids::DOC_INFO => Ok(self.info()),
            _ => {
                let result = self.editor.dispatch(id, &raw)?;
                if command_changes_document(id) {
                    self.mark_dirty();
                }
                Ok(result)
            }
        }
    }

    /// Document summary used by `doc.info` and the status bar.
    #[must_use]
    pub fn info(&self) -> Value {
        json!({
            "path": self.path.as_ref().map(|path| path.display().to_string()),
            "dirty": self.dirty,
            "pages": self.editor.doc.page_count(),
            "active_page": self.editor.doc.active_page,
            "shapes": self.editor.doc.shapes().len(),
            "selection": self.editor.selection.len(),
            "can_undo": self.editor.can_undo(),
            "can_redo": self.editor.can_redo(),
            "recent": self.recent.iter().map(|path| path.display().to_string()).collect::<Vec<_>>(),
        })
    }

    fn record_recent(&mut self, path: &Path) {
        let normalized = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        self.recent.retain(|candidate| candidate != &normalized);
        self.recent.insert(0, normalized);
        self.recent.truncate(MAX_RECENT);
        store_recent(&self.recent);
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

fn required_path(params: &Params<'_>) -> Result<PathBuf> {
    let path = params.string("path")?;
    if path.trim().is_empty() {
        return Err(SessionError::Invalid("path must not be empty".into()));
    }
    Ok(PathBuf::from(path))
}

fn parse_scope(params: &Params<'_>) -> Result<ExportScope> {
    let Some(value) = params.optional_string("scope") else {
        return Ok(ExportScope::Page);
    };
    ExportScope::parse(value)
        .ok_or_else(|| SessionError::Invalid(format!("unknown export scope {value}")))
}

fn command_changes_document(id: &str) -> bool {
    !(id.starts_with("view.")
        || id.starts_with("tool.")
        || matches!(
            id,
            ids::EDIT_SELECT_ALL | ids::EDIT_SELECT_NONE | ids::EDIT_COPY | ids::PAGE_LIST
        ))
}

fn config_dir() -> Option<PathBuf> {
    if let Some(value) = std::env::var_os("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(value).join("drawcraft"));
    }
    if let Some(value) = std::env::var_os("APPDATA") {
        return Some(PathBuf::from(value).join("drawcraft"));
    }
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config").join("drawcraft"))
}

fn recent_path() -> Option<PathBuf> {
    config_dir().map(|directory| directory.join("recent.json"))
}

fn load_recent() -> Vec<PathBuf> {
    let Some(path) = recent_path() else {
        return Vec::new();
    };
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let values: Vec<String> = serde_json::from_str(&text).unwrap_or_default();
    values
        .into_iter()
        .map(PathBuf::from)
        .take(MAX_RECENT)
        .collect()
}

fn store_recent(recent: &[PathBuf]) {
    let Some(path) = recent_path() else {
        return;
    };
    let Some(directory) = path.parent() else {
        return;
    };
    if std::fs::create_dir_all(directory).is_err() {
        return;
    }
    let values: Vec<String> = recent
        .iter()
        .map(|path| path.display().to_string())
        .collect();
    if let Ok(text) = serde_json::to_string_pretty(&values) {
        let _ = std::fs::write(path, text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use drawcraft_command::ids;
    use serde_json::json;

    fn empty_session() -> Session {
        Session::new()
    }

    #[test]
    fn commands_flow_through_session() {
        let mut session = empty_session();
        let value = session
            .execute(
                ids::SHAPE_ADD,
                &json!({"kind":"rectangle","x":10,"y":10,"width":50,"height":50}),
            )
            .unwrap_or_default();
        assert!(value.get("id").is_some());
        assert!(session.dirty);
        let info = session
            .execute(ids::DOC_INFO, &json!({}))
            .unwrap_or_default();
        assert_eq!(info.get("shapes").and_then(Value::as_u64), Some(1));
    }

    #[test]
    fn save_open_and_export_round_trip() {
        let directory = tempfile::tempdir().unwrap_or_else(|_| panic!("temp dir"));
        let path = directory.path().join("roundtrip.drawcraft");
        let mut session = empty_session();
        session
            .execute(
                ids::SHAPE_ADD,
                &json!({"kind":"ellipse","x":0,"y":0,"width":40,"height":40,"text":"hi"}),
            )
            .unwrap_or_default();
        session
            .execute(
                ids::FILE_SAVE_AS,
                &json!({"path": path.display().to_string()}),
            )
            .unwrap_or_else(|error| panic!("{error}"));
        assert!(!session.dirty);
        assert!(path.exists());

        let mut reopened = empty_session();
        reopened
            .execute(ids::FILE_OPEN, &json!({"path": path.display().to_string()}))
            .unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(reopened.editor.doc.shapes().len(), 1);

        let svg = directory.path().join("out.svg");
        reopened
            .execute(
                ids::FILE_EXPORT_SVG,
                &json!({"path": svg.display().to_string()}),
            )
            .unwrap_or_else(|error| panic!("{error}"));
        assert!(std::fs::metadata(svg).is_ok());
    }

    #[test]
    fn recent_is_deduped() {
        let mut session = empty_session();
        session.recent.clear();
        session.record_recent(Path::new("/tmp/a.drawcraft"));
        session.record_recent(Path::new("/tmp/b.drawcraft"));
        session.record_recent(Path::new("/tmp/a.drawcraft"));
        assert_eq!(session.recent.len(), 2);
        assert_eq!(session.recent[0], PathBuf::from("/tmp/a.drawcraft"));
    }
}
