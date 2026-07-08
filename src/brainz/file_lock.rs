//! OS advisory file locks for cross-process synchronization.
//!
//! Uses `fs4` for proper `flock` (Unix) / `LockFileEx` (Windows) semantics.
//! The kernel releases these locks automatically when the process dies,
//! preventing the stale-lock permadeath that sentinel-file approaches suffer.

use anyhow::{Context, Result};
use fs4::FileExt;
use std::fs::{self, OpenOptions};
use std::path::Path;

pub(super) struct FileLock {
    file: fs::File,
}

impl Drop for FileLock {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.file);
        // Keep the lockfile around — it's cheap and avoids recreating it.
        // The lock itself is released by unlock() above (and by the kernel on death).
    }
}

/// Acquire an exclusive advisory lock on `name` within the local-metrics dir.
/// Blocks until the lock is available or an error occurs.
pub(super) fn acquire(root: &Path, name: &str) -> Result<FileLock> {
    let dir = crate::dotdir::ensure(root, Some("local-metrics"))?;
    let path = dir.join(name);

    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .read(true)
        .open(&path)
        .with_context(|| format!("opening lock file {}", path.display()))?;

    FileExt::lock(&file)
        .with_context(|| format!("acquiring exclusive lock on {}", path.display()))?;

    Ok(FileLock { file })
}
