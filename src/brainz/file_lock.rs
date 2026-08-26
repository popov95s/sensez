//! OS advisory file locks for cross-process synchronization.
//!
//! Uses `fs4` for proper `flock` (Unix) / `LockFileEx` (Windows) semantics.
//! The kernel releases these locks automatically when the process dies,
//! preventing the stale-lock permadeath that sentinel-file approaches suffer.

use anyhow::{Context, Result};
use fs4::{FileExt, TryLockError};
use std::fs::{self, OpenOptions};
use std::path::Path;
use std::time::{Duration, Instant};

/// Poll interval while waiting for a peer process to release the lock.
const LOCK_POLL: Duration = Duration::from_millis(25);
/// How long to wait before telling the user we are blocked — a wedged peer
/// (SIGSTOP, NFS flock quirk) must not hang every sensez process silently.
const WARN_AFTER: Duration = Duration::from_secs(1);
/// Give up entirely after this long. Metrics are best-effort by contract
/// ("metrics must never fail the scan"), so timing out and reporting is
/// strictly better than blocking forever behind a stuck peer.
const GIVE_UP_AFTER: Duration = Duration::from_secs(10);

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
///
/// Waits at most [`GIVE_UP_AFTER`] for the holder, warning once after
/// [`WARN_AFTER`]. Callers treat an error here as "skip this metrics write",
/// never as a scan failure.
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

    let started = Instant::now();
    let mut warned = false;
    loop {
        match FileExt::try_lock(&file) {
            Ok(()) => return Ok(FileLock { file }),
            Err(TryLockError::WouldBlock) => {}
            Err(err) => {
                return Err(anyhow::Error::new(err))
                    .with_context(|| format!("acquiring exclusive lock on {}", path.display()))
            }
        }
        let waited = started.elapsed();
        if !warned && waited >= WARN_AFTER {
            warned = true;
            eprintln!(
                "[sensez metrics] waiting for {}: held by another sensez process?",
                path.display()
            );
        }
        if waited >= GIVE_UP_AFTER {
            anyhow::bail!(
                "gave up acquiring exclusive lock on {} after {}s (held by another process?)",
                path.display(),
                GIVE_UP_AFTER.as_secs()
            );
        }
        std::thread::sleep(LOCK_POLL);
    }
}
