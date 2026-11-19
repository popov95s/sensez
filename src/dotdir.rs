use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub fn ensure(root: &Path, sub: Option<&str>) -> Result<PathBuf> {
    let base = root.join(".sensez");
    let dir = match sub {
        Some(sub) => base.join(sub),
        None => base.clone(),
    };
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let ignore = base.join(".gitignore");
    if !ignore.exists() {
        std::fs::write(&ignore, "*\n").with_context(|| format!("writing {}", ignore.display()))?;
    }
    Ok(dir)
}

#[cfg(test)]
#[path = "dotdir_tests.rs"]
mod tests;
