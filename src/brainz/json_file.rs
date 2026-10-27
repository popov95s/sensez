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
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn corrupt_file_loads_default_and_keeps_backup() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("triage.json");
        fs::write(&path, "{not json").unwrap();

        let loaded: BTreeMap<String, u64> = load_or_default(&path);

        assert!(loaded.is_empty());
        assert_eq!(
            fs::read_to_string(tmp.path().join("triage.json.bak")).unwrap(),
            "{not json"
        );
    }

    #[test]
    fn missing_file_loads_default_without_backup() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("last-scan.json");

        let loaded: BTreeMap<String, u64> = load_or_default(&path);

        assert!(loaded.is_empty());
        assert!(!tmp.path().join("last-scan.json.bak").exists());
    }
}
