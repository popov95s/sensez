use std::path::{Path, PathBuf};

/// Write `body` to `dir/rel`, creating parent directories.
pub(crate) fn write(dir: &Path, rel: &str, body: &str) -> PathBuf {
    let path = dir.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, body).unwrap();
    path
}
