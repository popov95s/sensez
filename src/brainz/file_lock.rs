use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

const LOCK_TIMEOUT: Duration = Duration::from_secs(5);

pub(super) struct FileLock {
    path: PathBuf,
}

impl Drop for FileLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub(super) fn acquire(root: &Path, name: &str) -> Result<FileLock> {
    let dir = crate::dotdir::ensure(root, Some("local-metrics"))?;
    let path = dir.join(name);
    let start = Instant::now();
    loop {
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(_) => return Ok(FileLock { path }),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                if start.elapsed() >= LOCK_TIMEOUT {
                    anyhow::bail!("timed out waiting for {}", path.display());
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(err) => {
                return Err(err).with_context(|| format!("creating {}", path.display()));
            }
        }
    }
}
