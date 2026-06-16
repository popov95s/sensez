#[cfg(any(test, feature = "mcp", feature = "eyez"))]
use anyhow::{Context, Result};
#[cfg(any(test, feature = "mcp", feature = "eyez"))]
use std::path::{Path, PathBuf};

#[cfg(any(test, feature = "mcp", feature = "eyez"))]
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
mod tests {
    use super::*;

    #[test]
    fn creates_dir_and_self_ignore() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        std::fs::create_dir_all(&root).unwrap();

        let dir = ensure(&root, Some("local-metrics")).unwrap();
        assert!(dir.ends_with(".sensez/local-metrics") && dir.is_dir());
        let ignore = root.join(".sensez/.gitignore");
        assert_eq!(std::fs::read_to_string(&ignore).unwrap(), "*\n");

        std::fs::write(&ignore, "custom\n").unwrap();
        ensure(&root, None).unwrap();
        assert_eq!(std::fs::read_to_string(&ignore).unwrap(), "custom\n");
    }
}
