//! External-change detection via disk fingerprint polling.

use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::Read;
use std::path::Path;
use std::time::SystemTime;

use serde_json::{Value, json};

/// Snapshot of an on-disk file used to detect external edits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiskFingerprint {
    pub mtime: SystemTime,
    pub len: u64,
    pub hash: u64,
}

impl DiskFingerprint {
    /// Capture the fingerprint of `path`. Missing files return an error.
    pub fn capture(path: &Path) -> std::io::Result<Self> {
        let metadata = std::fs::metadata(path)?;
        let mtime = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        let len = metadata.len();
        let hash = hash_file(path)?;
        Ok(Self { mtime, len, hash })
    }

    /// Cheap check: compare metadata first, hash only when mtime or len changed.
    pub fn differs_from_disk(&self, path: &Path) -> std::io::Result<bool> {
        let metadata = match std::fs::metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(true),
            Err(error) => return Err(error),
        };
        let mtime = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        let len = metadata.len();
        if mtime == self.mtime && len == self.len {
            return Ok(false);
        }
        let hash = hash_file(path)?;
        Ok(hash != self.hash || len != self.len || mtime != self.mtime)
    }

    #[must_use]
    pub fn to_json(&self) -> Value {
        let mtime = self
            .mtime
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        json!({
            "mtime": mtime,
            "len": self.len,
            "hash": format!("{:016x}", self.hash),
        })
    }
}

fn hash_file(path: &Path) -> std::io::Result<u64> {
    let mut file = File::open(path)?;
    let mut buffer = [0_u8; 64 * 1024];
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        buffer[..read].hash(&mut hasher);
    }
    Ok(hasher.finish())
}

/// Stable hex fingerprint for autosave sidecars (not DefaultHasher).
#[must_use]
pub fn stable_content_hash(bytes: &[u8]) -> String {
    format!("{:016x}", fnv1a64(bytes))
}

#[must_use]
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;
    let mut hash = OFFSET;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}
