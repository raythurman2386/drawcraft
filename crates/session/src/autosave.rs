//! Debounced background autosave worker.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime};

use drawcraft_document::Document;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::watch::{DiskFingerprint, stable_content_hash};

/// Default debounce interval (30 s). Minimum allowed is 1 s.
pub const DEFAULT_INTERVAL_MS: u64 = 30_000;
pub const MIN_INTERVAL_MS: u64 = 1_000;
pub const EXTERNAL_POLL_MS: u64 = 2_000;

/// Autosave on/off and debounce interval.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AutosaveConfig {
    pub enabled: bool,
    pub interval_ms: u64,
}

impl Default for AutosaveConfig {
    fn default() -> Self {
        let interval_ms = std::env::var("DRAWCRAFT_AUTOSAVE_INTERVAL_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .map(|ms| ms.max(MIN_INTERVAL_MS))
            .unwrap_or(DEFAULT_INTERVAL_MS);
        Self {
            enabled: true,
            interval_ms,
        }
    }
}

impl AutosaveConfig {
    #[must_use]
    pub fn with_interval_ms(mut self, interval_ms: u64) -> Self {
        self.interval_ms = interval_ms.max(MIN_INTERVAL_MS);
        self
    }
}

/// Sidecar written next to each autosave `.drawcraft` file.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AutosaveMeta {
    pub source_path: Option<String>,
    pub source_mtime: Option<u64>,
    pub source_len: Option<u64>,
    pub source_hash: Option<String>,
    pub revision: u64,
    pub saved_at: u64,
    pub session_id: String,
}

/// Job sent to the background worker. Serialization and I/O happen off the UI thread.
struct AutosaveJob {
    document: Document,
    target: PathBuf,
    meta: AutosaveMeta,
    revision: u64,
}

enum WorkerMsg {
    Job(Box<AutosaveJob>),
    /// Drop any queued work so a finished save cannot be overwritten.
    Cancel,
    Shutdown,
}

/// Outcome of a background autosave write, polled by [`AutosaveRuntime::poll_results`].
pub enum WorkerResult {
    Done { revision: u64, path: PathBuf },
    Failed { revision: u64, error: String },
}

/// Owns the worker thread and debounce bookkeeping for one [`crate::Session`].
pub struct AutosaveRuntime {
    pub config: AutosaveConfig,
    pub last_autosaved_revision: u64,
    pub last_edit_at: Option<Instant>,
    pub last_autosave_at: Option<SystemTime>,
    pub last_autosave_path: Option<PathBuf>,
    pub pending: bool,
    /// Last background write failure message, cleared on the next success.
    pub last_error: Option<String>,
    cancel_generation: u64,
    /// Revision of the newest job still in flight / awaiting poll.
    queued_revision: Option<u64>,
    tx: Option<Sender<WorkerMsg>>,
    result_rx: Option<Receiver<WorkerResult>>,
    handle: Option<JoinHandle<()>>,
}

impl AutosaveRuntime {
    #[must_use]
    pub fn new(config: AutosaveConfig) -> Self {
        Self {
            config,
            last_autosaved_revision: 0,
            last_edit_at: None,
            last_autosave_at: None,
            last_autosave_path: None,
            pending: false,
            last_error: None,
            cancel_generation: 0,
            queued_revision: None,
            tx: None,
            result_rx: None,
            handle: None,
        }
    }

    fn ensure_worker(&mut self) {
        if self.tx.is_some() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        let (result_tx, result_rx) = mpsc::channel();
        let handle = std::thread::spawn(move || worker_loop(rx, result_tx));
        self.tx = Some(tx);
        self.result_rx = Some(result_rx);
        self.handle = Some(handle);
    }

    pub fn note_edit(&mut self, now: Instant) {
        self.last_edit_at = Some(now);
        self.pending = true;
    }

    pub fn cancel_pending(&mut self) {
        self.cancel_generation = self.cancel_generation.wrapping_add(1);
        self.pending = false;
        self.queued_revision = None;
        if let Some(tx) = &self.tx {
            let _ = tx.send(WorkerMsg::Cancel);
        }
    }

    /// Queue a background write when dirty long enough; returns the next wake delay.
    pub fn tick_debounce(
        &mut self,
        now: Instant,
        revision: u64,
        document: &Document,
        target: PathBuf,
        meta: AutosaveMeta,
    ) -> Option<Duration> {
        if !self.config.enabled {
            return None;
        }
        if revision == self.last_autosaved_revision && !self.pending {
            return None;
        }
        let Some(edited_at) = self.last_edit_at else {
            return Some(Duration::from_millis(self.config.interval_ms));
        };
        let interval = Duration::from_millis(self.config.interval_ms);
        let elapsed = now.saturating_duration_since(edited_at);
        if elapsed < interval {
            return Some(interval.saturating_sub(elapsed));
        }
        self.queue_job(revision, document.clone(), target, meta);
        None
    }

    fn queue_job(
        &mut self,
        revision: u64,
        document: Document,
        target: PathBuf,
        meta: AutosaveMeta,
    ) {
        self.ensure_worker();
        if let Some(tx) = &self.tx {
            let _ = tx.send(WorkerMsg::Job(Box::new(AutosaveJob {
                document,
                target,
                meta,
                revision,
            })));
            // Do not advance last_autosaved_revision or clear pending until the
            // worker reports success — failures must keep the revision pending.
            self.queued_revision = Some(revision);
        }
    }

    /// Apply worker outcomes. Returns the newest failure message this poll, if any.
    pub fn poll_results(&mut self) -> Option<String> {
        let Some(rx) = &self.result_rx else {
            return None;
        };
        let mut failure: Option<String> = None;
        loop {
            match rx.try_recv() {
                Ok(WorkerResult::Done { revision, path }) => {
                    if revision >= self.last_autosaved_revision {
                        self.last_autosaved_revision = revision;
                        self.last_autosave_path = Some(path);
                        self.last_autosave_at = Some(SystemTime::now());
                    }
                    self.last_error = None;
                    if self.queued_revision == Some(revision) {
                        self.pending = false;
                        self.queued_revision = None;
                    }
                }
                Ok(WorkerResult::Failed { revision, error }) => {
                    self.last_error = Some(error.clone());
                    self.pending = true;
                    if self.queued_revision == Some(revision) {
                        self.queued_revision = None;
                    }
                    failure = Some(error);
                }
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => break,
            }
        }
        failure
    }

    /// Synchronous autosave used by tests, CLI, and MCP.
    pub fn write_now(
        &mut self,
        revision: u64,
        document: &Document,
        target: &Path,
        meta: AutosaveMeta,
    ) -> crate::Result<PathBuf> {
        // Cancel any stale background write that could resurrect after this.
        self.cancel_pending();
        write_autosave_pair(document, target, &meta).inspect_err(|error| {
            self.last_error = Some(error.to_string());
            self.pending = true;
        })?;
        self.last_autosaved_revision = revision;
        self.last_autosave_path = Some(target.to_path_buf());
        self.last_autosave_at = Some(SystemTime::now());
        self.pending = false;
        self.last_error = None;
        self.queued_revision = None;
        Ok(target.to_path_buf())
    }

    pub fn status_json(&self, path: Option<&Path>) -> serde_json::Value {
        let last = self
            .last_autosave_at
            .and_then(|time| time.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs());
        json!({
            "enabled": self.config.enabled,
            "interval_ms": self.config.interval_ms,
            "path": path.map(|path| path.display().to_string()),
            "last_autosave_at": last,
            "pending": self.pending,
            "last_autosaved_revision": self.last_autosaved_revision,
            "last_error": self.last_error,
        })
    }

    pub fn shutdown(&mut self) {
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(WorkerMsg::Shutdown);
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for AutosaveRuntime {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn worker_loop(rx: Receiver<WorkerMsg>, result_tx: Sender<WorkerResult>) {
    loop {
        let first = match rx.recv() {
            Ok(WorkerMsg::Shutdown) | Err(_) => break,
            Ok(WorkerMsg::Cancel) => {
                drain_until_idle(&rx);
                continue;
            }
            Ok(WorkerMsg::Job(job)) => job,
        };
        let mut job = first;
        let mut cancelled = false;
        loop {
            match rx.try_recv() {
                Ok(WorkerMsg::Job(newer)) => job = newer,
                Ok(WorkerMsg::Cancel) => {
                    cancelled = true;
                    drain_until_idle(&rx);
                    break;
                }
                Ok(WorkerMsg::Shutdown) => return,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return,
            }
        }
        if cancelled {
            continue;
        }
        match write_autosave_pair(&job.document, &job.target, &job.meta) {
            Ok(()) => {
                let _ = result_tx.send(WorkerResult::Done {
                    revision: job.revision,
                    path: job.target,
                });
            }
            Err(error) => {
                let error = format!("{} ({})", error, job.target.display());
                let _ = result_tx.send(WorkerResult::Failed {
                    revision: job.revision,
                    error,
                });
            }
        }
    }
}

fn drain_until_idle(rx: &Receiver<WorkerMsg>) {
    while let Ok(msg) = rx.try_recv() {
        if matches!(msg, WorkerMsg::Shutdown) {
            // Caller loop will exit on the next recv after channel close; push
            // shutdown by dropping further work only.
            break;
        }
    }
}

pub fn write_autosave_pair(
    document: &Document,
    target: &Path,
    meta: &AutosaveMeta,
) -> crate::Result<()> {
    let json = drawcraft_format::to_json(document)?;
    drawcraft_format::write_atomic(target, json.as_bytes())?;
    let meta_path = meta_path_for(target);
    let meta_bytes = serde_json::to_vec_pretty(meta).map_err(|error| {
        crate::SessionError::Invalid(format!("autosave meta serialize: {error}"))
    })?;
    drawcraft_format::write_atomic(&meta_path, &meta_bytes)?;
    Ok(())
}

#[must_use]
pub fn meta_path_for(autosave_path: &Path) -> PathBuf {
    let mut path = autosave_path.to_path_buf();
    let file_name = autosave_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("autosave.drawcraft");
    path.set_file_name(format!("{file_name}.meta.json"));
    path
}

#[must_use]
pub fn autosave_dir(config_dir: &Path) -> PathBuf {
    config_dir.join("autosave")
}

/// Stable key for a document path, or `untitled-<session_id>` when unsaved.
#[must_use]
pub fn autosave_key(path: Option<&Path>, session_id: &str) -> String {
    match path {
        Some(path) => {
            let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
            format!(
                "{:016x}",
                crate::watch::fnv1a64(canonical.to_string_lossy().as_bytes())
            )
        }
        None => format!("untitled-{session_id}"),
    }
}

#[must_use]
pub fn autosave_path_for(config_dir: &Path, path: Option<&Path>, session_id: &str) -> PathBuf {
    autosave_dir(config_dir).join(format!("{}.drawcraft", autosave_key(path, session_id)))
}

#[must_use]
pub fn build_meta(
    source_path: Option<&Path>,
    revision: u64,
    session_id: &str,
    baseline: Option<&DiskFingerprint>,
) -> AutosaveMeta {
    let (source_mtime, source_len, source_hash) = match baseline {
        Some(fp) => {
            let mtime = fp
                .mtime
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|duration| duration.as_secs())
                .unwrap_or(0);
            (Some(mtime), Some(fp.len), Some(format!("{:016x}", fp.hash)))
        }
        None => {
            if let Some(path) = source_path {
                if let Ok(bytes) = std::fs::read(path) {
                    let meta = std::fs::metadata(path).ok();
                    let mtime = meta
                        .as_ref()
                        .and_then(|metadata| metadata.modified().ok())
                        .and_then(|time| time.duration_since(SystemTime::UNIX_EPOCH).ok())
                        .map(|duration| duration.as_secs());
                    let len = meta.map(|metadata| metadata.len());
                    (mtime, len, Some(stable_content_hash(&bytes)))
                } else {
                    (None, None, None)
                }
            } else {
                (None, None, None)
            }
        }
    };
    AutosaveMeta {
        source_path: source_path.map(|path| path.display().to_string()),
        source_mtime,
        source_len,
        source_hash,
        revision,
        saved_at: SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0),
        session_id: session_id.to_owned(),
    }
}

pub fn remove_autosave_pair(target: &Path) {
    let _ = std::fs::remove_file(target);
    let _ = std::fs::remove_file(meta_path_for(target));
}
