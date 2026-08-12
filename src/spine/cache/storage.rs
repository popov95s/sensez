use anyhow::{Context, Result};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

pub(super) fn atomic_write(path: &Path, bytes: &[u8], operation: &str) -> Result<()> {
    let Some(directory) = path.parent() else {
        return Ok(());
    };
    fs::create_dir_all(directory)?;
    let temp = path.with_extension(format!("tmp-{}", std::process::id()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&temp)?;
        file.write_all(bytes)?;
        fs::rename(&temp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result.with_context(|| operation.to_string())
}
