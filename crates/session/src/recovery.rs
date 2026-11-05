//! Crash-recovery candidate scan, restore, and discard.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use drawcraft_document::Document;
use serde_json::{Value, json};

use crate::SessionError;
use crate::autosave::{AutosaveMeta, meta_path_for};

/// One recoverable autosave.
#[derive(Clone, Debug)]
pub struct RecoveryCandidate {
    pub source_path: Option<PathBuf>,
    pub autosave_path: PathBuf,
    pub meta: AutosaveMeta,
    pub shapes: usize,
    pub autosave_mtime: SystemTime,
}

impl RecoveryCandidate {
    #[must_use]
    pub fn to_json(&self) -> Value {
        let autosave_mtime = self
            .autosave_mtime
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        json!({
            "source_path": self.source_path.as_ref().map(|path| path.display().to_string()),
            "autosave": self.autosave_path.display().to_string(),
            "saved_at": self.meta.saved_at,
            "autosave_mtime": autosave_mtime,
            "revision": self.meta.revision,
            "shapes": self.shapes,
        })
    }
}

/// Scan `autosave_dir` for candidates.
///
/// A candidate qualifies when the autosave is newer than the source mtime (when
/// a source exists) and its document content differs from the source. Equal
/// content is discarded silently (also neutralises a stale-worker race after save).
pub fn recovery_candidates(
    autosave_dir: &Path,
    filter_source: Option<&Path>,
) -> Vec<RecoveryCandidate> {
    let Ok(entries) = fs::read_dir(autosave_dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.ends_with(".drawcraft") || name.ends_with(".meta.json") {
            continue;
        }
        let meta_path = meta_path_for(&path);
        let Ok(meta_text) = fs::read_to_string(&meta_path) else {
            continue;
        };
        let Ok(meta) = serde_json::from_str::<AutosaveMeta>(&meta_text) else {
            continue;
        };
        let source_path = meta.source_path.as_ref().map(PathBuf::from);
        if let Some(filter) = filter_source {
            let matches = source_path
                .as_ref()
                .is_some_and(|source| paths_equal(source, filter));
            if !matches {
                continue;
            }
        }
        let Ok(autosave_meta) = fs::metadata(&path) else {
            continue;
        };
        let autosave_mtime = autosave_meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);

        if let Some(source) = source_path.as_ref()
            && source.exists()
        {
            if let Ok(source_meta) = fs::metadata(source)
                && let Ok(source_mtime) = source_meta.modified()
                && autosave_mtime <= source_mtime
            {
                continue;
            }
            if content_equal(&path, source) {
                // Stale worker finished after a successful save — drop silently.
                let _ = fs::remove_file(&path);
                let _ = fs::remove_file(&meta_path);
                continue;
            }
        }

        let shapes = drawcraft_format::load(&path)
            .map(|document| document.shapes().len())
            .unwrap_or(0);
        out.push(RecoveryCandidate {
            source_path,
            autosave_path: path,
            meta,
            shapes,
            autosave_mtime,
        });
    }
    out.sort_by_key(|left| std::cmp::Reverse(left.autosave_mtime));
    out
}

/// Load an autosave document for restore.
pub fn load_autosave(path: &Path) -> Result<(Document, AutosaveMeta), SessionError> {
    let document = drawcraft_format::load(path)?;
    let meta_text = fs::read_to_string(meta_path_for(path))?;
    let meta: AutosaveMeta = serde_json::from_str(&meta_text)
        .map_err(|error| SessionError::Invalid(format!("bad autosave meta: {error}")))?;
    Ok((document, meta))
}

pub fn discard_autosave(path: &Path) -> Result<(), SessionError> {
    let _ = fs::remove_file(path);
    let _ = fs::remove_file(meta_path_for(path));
    Ok(())
}

fn content_equal(autosave: &Path, source: &Path) -> bool {
    let Ok(auto_doc) = drawcraft_format::load(autosave) else {
        return false;
    };
    let Ok(source_doc) = drawcraft_format::load(source) else {
        return false;
    };
    auto_doc == source_doc
}

fn paths_equal(left: &Path, right: &Path) -> bool {
    let left = fs::canonicalize(left).unwrap_or_else(|_| left.to_path_buf());
    let right = fs::canonicalize(right).unwrap_or_else(|_| right.to_path_buf());
    left == right
}
