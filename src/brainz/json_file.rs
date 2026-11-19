//! Lenient JSON loading for metrics files that must never fail a scan.

use serde::de::DeserializeOwned;
use std::fs;
use std::path::Path;

/// Parse `path`, or return the default; an unreadable file is copied to `<path>.bak` with one stderr note.
pub fn load_or_default<T: DeserializeOwned + Default>(path: &Path) -> T {
    let Ok(bytes) = fs::read(path) else {
        return T::default();
    };
    serde_json::from_slice(&bytes).unwrap_or_else(|err| {
        keep_backup(path, &bytes, &err);
        T::default()
    })
}

fn keep_backup(path: &Path, bytes: &[u8], err: &serde_json::Error) {
    let backup = path.with_extension("json.bak");
    if fs::read(&backup).is_ok_and(|kept| kept == bytes) {
        return;
    }
    let saved = fs::write(&backup, bytes).map_or_else(
        |write_err| format!("backup failed: {write_err}"),
        |()| format!("copy kept at {}", backup.display()),
    );
    eprintln!(
        "[sensez] {} unreadable, ignoring ({err}); {saved}",
        path.display()
    );
}

#[cfg(test)]
#[path = "json_file_tests.rs"]
mod tests;
