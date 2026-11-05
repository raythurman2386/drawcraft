//! Workspace session: the bridge between the headless editor and files.
//!
//! `Session` owns an [`Editor`] plus the current path, dirty flag, recent-file
//! list, autosave/recovery/external-change state, and export helpers. The UI,
//! CLI, JSON channel, and MCP server all use the same [`Session::execute`]
//! entry point.

#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::field_reassign_with_default
    )
)]

mod autosave;
mod recovery;
mod watch;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime};

use drawcraft_command::ids;
use drawcraft_editor::{EditError, Editor, Params};
use drawcraft_format::{ExportScope, FormatError};
use serde_json::{Value, json};
use thiserror::Error;

pub use autosave::{
    AutosaveConfig, AutosaveMeta, DEFAULT_INTERVAL_MS, EXTERNAL_POLL_MS, MIN_INTERVAL_MS,
};
pub use recovery::RecoveryCandidate;
pub use watch::DiskFingerprint;

const MAX_RECENT: usize = 12;
static SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

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
    #[error("file changed on disk: {}", .path.display())]
    ExternalChange { path: PathBuf },
    #[error("document has unsaved changes; pass force=true to reload")]
    DirtyReload,
}

pub type Result<T> = std::result::Result<T, SessionError>;

/// Outcome of [`Session::tick`].
#[derive(Clone, Debug, Default)]
pub struct TickOutcome {
    /// Ask the UI to wake after this delay so timers keep firing while idle.
    pub next_wake: Option<Duration>,
    /// True when the on-disk file no longer matches the baseline fingerprint.
    pub external_changed: bool,
}

/// A document open in the workspace.
pub struct Session {
    pub editor: Editor,
    pub path: Option<PathBuf>,
    pub dirty: bool,
    /// Monotonic edit counter bumped by [`Self::mark_dirty`].
    pub revision: u64,
    recent: Vec<PathBuf>,
    config_dir: PathBuf,
    session_id: String,
    autosave: autosave::AutosaveRuntime,
    disk_baseline: Option<DiskFingerprint>,
    last_external_poll: Option<Instant>,
    /// When true, the next successful save skips the external-change re-check
    /// because [`Self::keep_mine`] already accepted the disk state.
    keep_mine_armed: bool,
}

impl Session {
    #[must_use]
    pub fn new() -> Self {
        Self::with_config_dir(default_config_dir())
    }

    /// Construct a session that stores recent/autosave state under `config_dir`.
    #[must_use]
    pub fn with_config_dir(config_dir: impl Into<PathBuf>) -> Self {
        let config_dir = config_dir.into();
        let recent = load_recent(&config_dir);
        let session_id = format!(
            "{}-{}",
            std::process::id(),
            SESSION_COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        Self {
            editor: Editor::new(),
            path: None,
            dirty: false,
            revision: 0,
            recent,
            config_dir,
            session_id,
            autosave: autosave::AutosaveRuntime::new(AutosaveConfig::default()),
            disk_baseline: None,
            last_external_poll: None,
            keep_mine_armed: false,
        }
    }

    #[must_use]
    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    #[must_use]
    pub fn recent(&self) -> &[PathBuf] {
        &self.recent
    }

    /// Recent entries with an `exists` flag for the Open Recent UI.
    #[must_use]
    pub fn recent_entries(&self) -> Vec<(PathBuf, bool)> {
        self.recent
            .iter()
            .map(|path| (path.clone(), path.exists()))
            .collect()
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

    #[must_use]
    pub fn autosave_path(&self) -> PathBuf {
        autosave::autosave_path_for(&self.config_dir, self.path.as_deref(), &self.session_id)
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
        self.revision = self.revision.wrapping_add(1);
        self.autosave.note_edit(Instant::now());
    }

    /// Drive autosave debounce and external-change polling.
    pub fn tick(&mut self, now: Instant) -> TickOutcome {
        let mut outcome = TickOutcome::default();

        let target = self.autosave_path();
        let meta = autosave::build_meta(
            self.path.as_deref(),
            self.revision,
            &self.session_id,
            self.disk_baseline.as_ref(),
        );
        outcome.next_wake =
            self.autosave
                .tick_debounce(now, self.revision, &self.editor.doc, target, meta);

        let poll = Duration::from_millis(EXTERNAL_POLL_MS);
        let due = self
            .last_external_poll
            .is_none_or(|last| now.saturating_duration_since(last) >= poll);
        if due {
            self.last_external_poll = Some(now);
            if let Ok(check) = self.external_check() {
                outcome.external_changed = check
                    .get("changed")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
            }
            let remaining = outcome.next_wake.unwrap_or(poll).min(poll);
            outcome.next_wake = Some(remaining);
        } else if let Some(last) = self.last_external_poll {
            let remaining = poll.saturating_sub(now.saturating_duration_since(last));
            outcome.next_wake = Some(match outcome.next_wake {
                Some(wake) => wake.min(remaining),
                None => remaining,
            });
        }

        outcome
    }

    /// Reset to a blank document.
    pub fn new_document(&mut self) {
        self.clear_current_autosave();
        self.editor.reset();
        self.path = None;
        self.dirty = false;
        self.disk_baseline = None;
        self.keep_mine_armed = false;
        self.session_id = format!(
            "{}-{}",
            std::process::id(),
            SESSION_COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        self.autosave.last_autosaved_revision = self.revision;
        self.autosave.pending = false;
    }

    /// Open a document and make it current.
    pub fn open(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        let document = drawcraft_format::load(path)?;
        self.clear_current_autosave();
        self.editor.replace_document(document)?;
        self.path = Some(path.to_path_buf());
        self.dirty = false;
        self.disk_baseline = DiskFingerprint::capture(path).ok();
        self.keep_mine_armed = false;
        self.autosave.last_autosaved_revision = self.revision;
        self.autosave.pending = false;
        self.record_recent(path);
        Ok(())
    }

    /// Save to the current path.
    pub fn save(&mut self) -> Result<PathBuf> {
        self.save_with_force(false)
    }

    pub fn save_with_force(&mut self, force: bool) -> Result<PathBuf> {
        let path = self.path.clone().ok_or(SessionError::NoPath)?;
        self.save_to(&path, force)?;
        Ok(path)
    }

    /// Save to an explicit path and make it current.
    pub fn save_as(&mut self, path: impl AsRef<Path>) -> Result<PathBuf> {
        self.save_as_with_force(path, false)
    }

    pub fn save_as_with_force(&mut self, path: impl AsRef<Path>, force: bool) -> Result<PathBuf> {
        let path = path.as_ref().to_path_buf();
        self.save_to(&path, force)?;
        self.path = Some(path.clone());
        self.record_recent(&path);
        Ok(path)
    }

    fn save_to(&mut self, path: &Path, force: bool) -> Result<()> {
        if !force
            && !self.keep_mine_armed
            && let Some(baseline) = &self.disk_baseline
            && path.exists()
            && baseline.differs_from_disk(path)?
        {
            return Err(SessionError::ExternalChange {
                path: path.to_path_buf(),
            });
        }
        drawcraft_format::save(&self.editor.doc, path)?;
        self.dirty = false;
        self.disk_baseline = DiskFingerprint::capture(path).ok();
        self.keep_mine_armed = false;
        self.autosave.cancel_pending();
        self.clear_current_autosave();
        self.autosave.last_autosaved_revision = self.revision;
        Ok(())
    }

    /// Open the last saved document path.
    pub fn reload(&mut self) -> Result<()> {
        self.reload_with_force(false)
    }

    pub fn reload_with_force(&mut self, force: bool) -> Result<()> {
        if self.dirty && !force {
            return Err(SessionError::DirtyReload);
        }
        let path = self.path.clone().ok_or(SessionError::NoPath)?;
        self.open(path)
    }

    /// Accept the current disk fingerprint so the next save overwrites deliberately.
    pub fn keep_mine(&mut self) -> Result<Value> {
        let path = self.path.clone().ok_or(SessionError::NoPath)?;
        self.disk_baseline = DiskFingerprint::capture(&path).ok();
        self.keep_mine_armed = true;
        Ok(json!({
            "ok": true,
            "path": path.display().to_string(),
            "dirty": self.dirty,
        }))
    }

    pub fn external_check(&self) -> Result<Value> {
        let Some(path) = self.path.as_ref() else {
            return Ok(json!({
                "changed": false,
                "path": Value::Null,
                "disk_mtime": Value::Null,
            }));
        };
        let Some(baseline) = &self.disk_baseline else {
            return Ok(json!({
                "changed": false,
                "path": path.display().to_string(),
                "disk_mtime": Value::Null,
            }));
        };
        let changed = baseline.differs_from_disk(path)?;
        let disk_mtime = std::fs::metadata(path)
            .ok()
            .and_then(|metadata| metadata.modified().ok())
            .and_then(|time| time.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs());
        Ok(json!({
            "changed": changed,
            "path": path.display().to_string(),
            "disk_mtime": disk_mtime,
        }))
    }

    pub fn autosave_status(&self) -> Value {
        self.autosave.status_json(Some(&self.autosave_path()))
    }

    pub fn autosave_set(&mut self, enabled: Option<bool>, interval_ms: Option<u64>) -> Value {
        if let Some(enabled) = enabled {
            self.autosave.config.enabled = enabled;
        }
        if let Some(interval_ms) = interval_ms {
            self.autosave.config.interval_ms = interval_ms.max(MIN_INTERVAL_MS);
        }
        self.autosave_status()
    }

    pub fn autosave_now(&mut self) -> Result<Value> {
        let target = self.autosave_path();
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let meta = autosave::build_meta(
            self.path.as_deref(),
            self.revision,
            &self.session_id,
            self.disk_baseline.as_ref(),
        );
        let path = self
            .autosave
            .write_now(self.revision, &self.editor.doc, &target, meta)?;
        Ok(json!({"path": path.display().to_string(), "revision": self.revision}))
    }

    #[must_use]
    pub fn list_recovery_candidates(&self) -> Vec<RecoveryCandidate> {
        let dir = autosave::autosave_dir(&self.config_dir);
        recovery::recovery_candidates(&dir, None)
    }

    pub fn recover_list(&self) -> Value {
        let candidates = self.list_recovery_candidates();
        json!({
            "candidates": candidates.iter().map(RecoveryCandidate::to_json).collect::<Vec<_>>(),
        })
    }

    pub fn recover(&mut self, action: &str, autosave: Option<&Path>) -> Result<Value> {
        let dir = autosave::autosave_dir(&self.config_dir);
        let candidates = recovery::recovery_candidates(&dir, None);
        let candidate = match autosave {
            Some(path) => candidates
                .into_iter()
                .find(|candidate| candidate.autosave_path == path)
                .ok_or_else(|| SessionError::Invalid("autosave candidate not found".into()))?,
            None => candidates
                .into_iter()
                .next()
                .ok_or_else(|| SessionError::Invalid("no recovery candidates".into()))?,
        };
        match action {
            "discard" => {
                recovery::discard_autosave(&candidate.autosave_path)?;
                Ok(json!({"ok": true, "action": "discard"}))
            }
            "restore" => {
                let (document, _meta) = recovery::load_autosave(&candidate.autosave_path)?;
                let source = candidate.source_path.clone();
                self.editor.replace_document(document)?;
                self.path = source.clone();
                self.dirty = true;
                self.revision = self.revision.wrapping_add(1);
                self.autosave.note_edit(Instant::now());
                self.disk_baseline = source
                    .as_ref()
                    .and_then(|path| DiskFingerprint::capture(path).ok());
                self.keep_mine_armed = false;
                // Keep the autosave until the next successful save.
                Ok(json!({
                    "ok": true,
                    "action": "restore",
                    "path": self.path.as_ref().map(|path| path.display().to_string()),
                    "dirty": true,
                    "shapes": self.editor.doc.shapes().len(),
                }))
            }
            other => Err(SessionError::Invalid(format!(
                "unknown recover action {other}"
            ))),
        }
    }

    pub fn clear_recent(&mut self) -> Value {
        self.recent.clear();
        store_recent(&self.config_dir, &self.recent);
        json!({"ok": true, "recent": []})
    }

    pub fn remove_recent(&mut self, path: &Path) -> Value {
        let normalized = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        self.recent.retain(|candidate| {
            let other = std::fs::canonicalize(candidate).unwrap_or_else(|_| candidate.clone());
            other != normalized && candidate != path
        });
        store_recent(&self.config_dir, &self.recent);
        json!({
            "ok": true,
            "recent": self.recent.iter().map(|path| path.display().to_string()).collect::<Vec<_>>(),
        })
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

    /// Execute a command ID. File/export/lifecycle commands are handled here;
    /// everything else delegates to the editor.
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
                let force = params.boolean_or("force", false);
                if let Some(path) = params.optional_string("path") {
                    let path = self.save_as_with_force(path, force)?;
                    Ok(json!({"path": path}))
                } else {
                    let path = self.save_with_force(force)?;
                    Ok(json!({"path": path}))
                }
            }
            ids::FILE_SAVE_AS => {
                let path = required_path(&params)?;
                let force = params.boolean_or("force", false);
                let path = self.save_as_with_force(path, force)?;
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
                "recent": self.recent.iter().map(|path| path.display().to_string()).collect::<Vec<_>>(),
                "entries": self.recent_entries().into_iter().map(|(path, exists)| {
                    json!({"path": path.display().to_string(), "exists": exists})
                }).collect::<Vec<_>>(),
            })),
            ids::DOC_RECENT_CLEAR => Ok(self.clear_recent()),
            ids::DOC_RECENT_REMOVE => {
                let path = required_path(&params)?;
                Ok(self.remove_recent(&path))
            }
            ids::DOC_INFO => Ok(self.info()),
            ids::SESSION_AUTOSAVE_STATUS => Ok(self.autosave_status()),
            ids::SESSION_AUTOSAVE_SET => {
                let enabled = params.get("enabled").and_then(Value::as_bool);
                let interval_ms = params
                    .get("interval_ms")
                    .and_then(Value::as_i64)
                    .map(|value| value.max(0) as u64);
                Ok(self.autosave_set(enabled, interval_ms))
            }
            ids::SESSION_AUTOSAVE_NOW => self.autosave_now(),
            ids::SESSION_RECOVER_LIST => Ok(self.recover_list()),
            ids::SESSION_RECOVER => {
                let action = params.string("action")?;
                let autosave = params.optional_string("autosave").map(PathBuf::from);
                self.recover(action, autosave.as_deref())
            }
            ids::SESSION_EXTERNAL_CHECK => self.external_check(),
            ids::SESSION_RELOAD => {
                let force = params.boolean_or("force", false);
                self.reload_with_force(force)?;
                Ok(self.info())
            }
            ids::SESSION_KEEP_MINE => self.keep_mine(),
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
            "revision": self.revision,
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
        store_recent(&self.config_dir, &self.recent);
    }

    fn clear_current_autosave(&mut self) {
        let path = self.autosave_path();
        autosave::remove_autosave_pair(&path);
        // Also clear any path keyed by the current source (after path changes).
        if let Some(source) = self.path.as_ref() {
            let keyed =
                autosave::autosave_path_for(&self.config_dir, Some(source), &self.session_id);
            autosave::remove_autosave_pair(&keyed);
        }
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // Clean exit with a saved document removes its autosave.
        if !self.dirty {
            self.clear_current_autosave();
        }
        self.autosave.shutdown();
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

fn default_config_dir() -> PathBuf {
    if let Some(value) = std::env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(value).join("drawcraft");
    }
    if let Some(value) = std::env::var_os("APPDATA") {
        return PathBuf::from(value).join("drawcraft");
    }
    std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join(".config").join("drawcraft"))
        .unwrap_or_else(|| PathBuf::from(".drawcraft-config"))
}

fn recent_path(config_dir: &Path) -> PathBuf {
    config_dir.join("recent.json")
}

fn load_recent(config_dir: &Path) -> Vec<PathBuf> {
    let path = recent_path(config_dir);
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

fn store_recent(config_dir: &Path, recent: &[PathBuf]) {
    let path = recent_path(config_dir);
    if std::fs::create_dir_all(config_dir).is_err() {
        return;
    }
    let values: Vec<String> = recent
        .iter()
        .map(|path| path.display().to_string())
        .collect();
    if let Ok(text) = serde_json::to_string_pretty(&values) {
        let _ = drawcraft_format::write_atomic(&path, text.as_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use drawcraft_command::{Scope, commands, ids};
    use serde_json::json;
    use std::time::Duration;

    fn test_session() -> (tempfile::TempDir, Session) {
        let directory = tempfile::tempdir().unwrap_or_else(|_| panic!("temp dir"));
        let session = Session::with_config_dir(directory.path().join("cfg"));
        (directory, session)
    }

    #[test]
    fn commands_flow_through_session() {
        let (_dir, mut session) = test_session();
        let value = session
            .execute(
                ids::SHAPE_ADD,
                &json!({"kind":"rectangle","x":10,"y":10,"width":50,"height":50}),
            )
            .unwrap_or_default();
        assert!(value.get("id").is_some());
        assert!(session.dirty);
        assert!(session.revision >= 1);
        let info = session
            .execute(ids::DOC_INFO, &json!({}))
            .unwrap_or_default();
        assert_eq!(info.get("shapes").and_then(Value::as_u64), Some(1));
    }

    #[test]
    fn save_open_and_export_round_trip() {
        let (directory, mut session) = test_session();
        let path = directory.path().join("roundtrip.drawcraft");
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

        let mut reopened = Session::with_config_dir(directory.path().join("cfg2"));
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
        let (directory, mut session) = test_session();
        session.recent.clear();
        let a = directory.path().join("a.drawcraft");
        let b = directory.path().join("b.drawcraft");
        std::fs::write(&a, "{}").unwrap();
        std::fs::write(&b, "{}").unwrap();
        session.record_recent(&a);
        session.record_recent(&b);
        session.record_recent(&a);
        assert_eq!(session.recent.len(), 2);
        assert_eq!(
            session.recent[0],
            std::fs::canonicalize(&a).unwrap_or(a.clone())
        );
        // Injected config dir — never touch the real user config.
        assert!(directory.path().join("cfg").join("recent.json").exists());
    }

    #[test]
    fn autosave_now_and_recover_restore_equals_document() {
        let (directory, mut session) = test_session();
        let path = directory.path().join("board.drawcraft");
        session
            .execute(
                ids::SHAPE_ADD,
                &json!({"kind":"rectangle","x":1,"y":2,"width":30,"height":40}),
            )
            .unwrap();
        session
            .execute(
                ids::FILE_SAVE_AS,
                &json!({"path": path.display().to_string()}),
            )
            .unwrap();
        session
            .execute(
                ids::SHAPE_ADD,
                &json!({"kind":"ellipse","x":10,"y":10,"width":20,"height":20}),
            )
            .unwrap();
        let expected = session.editor.doc.clone();
        session
            .execute(ids::SESSION_AUTOSAVE_NOW, &json!({}))
            .unwrap();

        // Simulate crash: drop without saving, reopen path, restore.
        drop(session);
        let mut session = Session::with_config_dir(directory.path().join("cfg"));
        session.open(&path).unwrap();
        let list = session.recover_list();
        let candidates = list
            .get("candidates")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(candidates.len(), 1);
        session
            .execute(ids::SESSION_RECOVER, &json!({"action": "restore"}))
            .unwrap();
        assert_eq!(session.editor.doc, expected);
        assert!(session.dirty);
        assert_eq!(session.path.as_deref(), Some(path.as_path()));
    }

    #[test]
    fn recover_discard_removes_autosave() {
        let (directory, mut session) = test_session();
        let path = directory.path().join("board.drawcraft");
        session
            .execute(
                ids::SHAPE_ADD,
                &json!({"kind":"rectangle","x":0,"y":0,"width":10,"height":10}),
            )
            .unwrap();
        session
            .execute(
                ids::FILE_SAVE_AS,
                &json!({"path": path.display().to_string()}),
            )
            .unwrap();
        session
            .execute(
                ids::SHAPE_ADD,
                &json!({"kind":"rectangle","x":5,"y":5,"width":5,"height":5}),
            )
            .unwrap();
        let autosave = session
            .execute(ids::SESSION_AUTOSAVE_NOW, &json!({}))
            .unwrap();
        let autosave_path = autosave
            .get("path")
            .and_then(Value::as_str)
            .unwrap()
            .to_owned();
        drop(session);

        let mut session = Session::with_config_dir(directory.path().join("cfg"));
        session
            .execute(ids::SESSION_RECOVER, &json!({"action": "discard"}))
            .unwrap();
        assert!(!PathBuf::from(&autosave_path).exists());
        let list = session.recover_list();
        assert!(
            list.get("candidates")
                .and_then(Value::as_array)
                .is_some_and(Vec::is_empty)
        );
    }

    #[test]
    fn external_change_blocks_save_until_force_or_keep_mine() {
        let (directory, mut session) = test_session();
        let path = directory.path().join("watched.drawcraft");
        session
            .execute(
                ids::SHAPE_ADD,
                &json!({"kind":"rectangle","x":0,"y":0,"width":10,"height":10}),
            )
            .unwrap();
        session
            .execute(
                ids::FILE_SAVE_AS,
                &json!({"path": path.display().to_string()}),
            )
            .unwrap();

        // External rewrite with forced newer mtime.
        let mut outsider = Session::with_config_dir(directory.path().join("other"));
        outsider
            .execute(
                ids::SHAPE_ADD,
                &json!({"kind":"ellipse","x":0,"y":0,"width":8,"height":8}),
            )
            .unwrap();
        outsider
            .execute(
                ids::FILE_SAVE_AS,
                &json!({"path": path.display().to_string()}),
            )
            .unwrap();
        let newer = SystemTime::now() + Duration::from_secs(5);
        let file = std::fs::File::options().write(true).open(&path).unwrap();
        file.set_modified(newer).unwrap();

        let check = session.external_check().unwrap();
        assert_eq!(check.get("changed").and_then(Value::as_bool), Some(true));

        let err = session.execute(ids::FILE_SAVE, &json!({})).unwrap_err();
        assert!(matches!(err, SessionError::ExternalChange { .. }));

        session
            .execute(ids::FILE_SAVE, &json!({"force": true}))
            .unwrap();
        assert!(!session.dirty);

        // Reload path: dirty reload refuses without force.
        session
            .execute(
                ids::SHAPE_ADD,
                &json!({"kind":"triangle","x":1,"y":1,"width":4,"height":4}),
            )
            .unwrap();
        // External change again.
        std::fs::write(
            &path,
            drawcraft_format::to_json(&outsider.editor.doc).unwrap(),
        )
        .unwrap();
        let file = std::fs::File::options().write(true).open(&path).unwrap();
        file.set_modified(SystemTime::now() + Duration::from_secs(10))
            .unwrap();
        session.keep_mine().unwrap();
        session.execute(ids::FILE_SAVE, &json!({})).unwrap();
    }

    #[test]
    fn reload_and_keep_mine_paths() {
        let (directory, mut session) = test_session();
        let path = directory.path().join("reload.drawcraft");
        session
            .execute(
                ids::SHAPE_ADD,
                &json!({"kind":"rectangle","x":0,"y":0,"width":10,"height":10}),
            )
            .unwrap();
        session
            .execute(
                ids::FILE_SAVE_AS,
                &json!({"path": path.display().to_string()}),
            )
            .unwrap();
        session
            .execute(
                ids::SHAPE_ADD,
                &json!({"kind":"ellipse","x":0,"y":0,"width":3,"height":3}),
            )
            .unwrap();
        assert!(matches!(
            session.execute(ids::SESSION_RELOAD, &json!({})),
            Err(SessionError::DirtyReload)
        ));
        session
            .execute(ids::SESSION_RELOAD, &json!({"force": true}))
            .unwrap();
        assert_eq!(session.editor.doc.shapes().len(), 1);
        assert!(!session.dirty);
    }

    #[test]
    fn recent_clear_and_remove() {
        let (directory, mut session) = test_session();
        let a = directory.path().join("a.drawcraft");
        let b = directory.path().join("b.drawcraft");
        std::fs::write(&a, "{}").unwrap();
        std::fs::write(&b, "{}").unwrap();
        session.record_recent(&a);
        session.record_recent(&b);
        let listed = session.execute(ids::DOC_RECENT, &json!({})).unwrap();
        assert_eq!(
            listed
                .get("entries")
                .and_then(Value::as_array)
                .map(Vec::len),
            Some(2)
        );
        session
            .execute(
                ids::DOC_RECENT_REMOVE,
                &json!({"path": a.display().to_string()}),
            )
            .unwrap();
        assert_eq!(session.recent.len(), 1);
        session.execute(ids::DOC_RECENT_CLEAR, &json!({})).unwrap();
        assert!(session.recent.is_empty());
    }

    #[test]
    fn every_session_command_is_handled_by_execute() {
        let (_dir, mut session) = test_session();
        for command in commands()
            .iter()
            .filter(|command| command.scope == Scope::Session)
        {
            let result = session.execute(command.id, &json!({}));
            if let Err(SessionError::Editor(EditError::UnknownCommand(_))) = result {
                panic!("{} fell through to editor UnknownCommand", command.id);
            }
        }
    }

    #[test]
    fn equal_content_autosave_is_discarded_silently() {
        let (directory, mut session) = test_session();
        let path = directory.path().join("same.drawcraft");
        session
            .execute(
                ids::SHAPE_ADD,
                &json!({"kind":"rectangle","x":0,"y":0,"width":10,"height":10}),
            )
            .unwrap();
        session
            .execute(
                ids::FILE_SAVE_AS,
                &json!({"path": path.display().to_string()}),
            )
            .unwrap();
        // Force an autosave of the same content after save (stale-worker race).
        session.dirty = true;
        session.revision += 1;
        session
            .execute(ids::SESSION_AUTOSAVE_NOW, &json!({}))
            .unwrap();
        // Make autosave appear newer.
        let autosave = session.autosave_path();
        let file = std::fs::File::options()
            .write(true)
            .open(&autosave)
            .unwrap();
        file.set_modified(SystemTime::now() + Duration::from_secs(30))
            .unwrap();
        let list = session.recover_list();
        assert!(
            list.get("candidates")
                .and_then(Value::as_array)
                .is_some_and(Vec::is_empty)
        );
    }
}
