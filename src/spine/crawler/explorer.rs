//! Parallel source discovery using the `ignore` crate's threaded walker.

use super::generated;
use crate::globs::build_globset;
use crate::profiles::registry;
use anyhow::{anyhow, Result};
use ignore::{WalkBuilder, WalkState};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};

/// Result of a discovery walk: the source files found, plus how many entries
/// could not be read. A non-zero `skipped` means the scan is incomplete — it
/// is surfaced in the report metadata so it is never mistaken for "clean".
#[derive(Debug, Default)]
pub struct Discovery {
    pub files: Vec<PathBuf>,
    pub skipped: usize,
}

/// Walk `root` in parallel, returning every source file whose extension is
/// claimed by a compiled-in language profile, minus any `exclude`-glob match.
///
/// `.gitignore`, `.ignore`, and hidden-file rules are respected by default.
/// I/O errors on individual entries are counted, not silently dropped.
pub fn collect_source_files(root: &Path, exclude: &[String]) -> Result<Discovery> {
    if !root.exists() {
        return Err(anyhow!("path does not exist: {}", root.display()));
    }
    let excludes = build_globset(exclude);
    let skipped = Arc::new(AtomicUsize::new(0));

    let (tx, rx) = mpsc::channel::<PathBuf>();
    WalkBuilder::new(root).build_parallel().run(|| {
        let tx = tx.clone();
        let excludes = excludes.clone();
        let skipped = Arc::clone(&skipped);
        Box::new(move |entry| {
            match entry {
                Ok(entry) => {
                    let path = entry.path();
                    if is_source_file(&entry)
                        && !excludes.is_match(path)
                        && !generated::is_generated_or_data_source(path)
                    {
                        // Receiver lives until the walk completes; ignore send races.
                        let _ = tx.send(entry.into_path());
                    }
                }
                Err(_) => {
                    skipped.fetch_add(1, Ordering::Relaxed);
                }
            }
            WalkState::Continue
        })
    });
    drop(tx);

    let mut files: Vec<PathBuf> = rx.into_iter().collect();
    files.sort();
    Ok(Discovery {
        files,
        skipped: skipped.load(Ordering::Relaxed),
    })
}

/// True for regular files whose extension a language profile claims.
fn is_source_file(entry: &ignore::DirEntry) -> bool {
    entry.file_type().is_some_and(|ft| ft.is_file())
        && registry::parse_for_path(entry.path()).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn finds_py_files_and_skips_others() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_path_buf();
        fs::create_dir_all(dir.join("pkg")).unwrap();
        fs::write(dir.join("a.py"), "x = 1\n").unwrap();
        fs::write(dir.join("pkg/b.py"), "y = 2\n").unwrap();
        fs::write(dir.join("notes.txt"), "ignore me\n").unwrap();

        let found = collect_source_files(&dir, &[]).unwrap();
        assert_eq!(found.skipped, 0, "readable tree skips nothing");
        let names: Vec<_> = found
            .files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(names.contains(&"a.py".to_string()));
        assert!(names.contains(&"b.py".to_string()));
        assert!(!names.iter().any(|n| n.ends_with(".txt")));

        // exclude glob drops matching files from discovery
        let filtered = collect_source_files(&dir, &["**/pkg/**".to_string()]).unwrap();
        let fnames: Vec<_> = filtered
            .files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(fnames.contains(&"a.py".to_string()));
        assert!(!fnames.contains(&"b.py".to_string()), "pkg/ excluded");
    }

    #[test]
    fn missing_path_errors() {
        assert!(collect_source_files(Path::new("/nonexistent/sensez/xyz"), &[]).is_err());
    }
}
