//! Shared package/crate-root detection for path-keyed languages (JS/TS, Rust).
//!
//! Both key modules by root-relative file path; they differ only in the
//! marker file (`package.json` vs `Cargo.toml`), the vendor boundaries, and
//! the per-language key rule (extension strip + index collapse), which each
//! language applies to the segments returned by [`rel_parts`].

use std::path::{Path, PathBuf};

/// Nearest ancestor of `file` containing one of `markers`, without crossing a
/// `boundaries` directory; else the file's own directory.
pub(crate) fn root_for(file: &Path, markers: &[&str], boundaries: &[&str]) -> PathBuf {
    let start = file.parent().unwrap_or(Path::new("."));
    let mut dir = Some(start);
    while let Some(d) = dir {
        if d.file_name()
            .is_some_and(|n| boundaries.contains(&n.to_string_lossy().as_ref()))
        {
            break;
        }
        if markers.iter().any(|m| d.join(m).exists()) {
            return d.to_path_buf();
        }
        dir = d.parent();
    }
    start.to_path_buf()
}

/// Root-relative path segments of `file` (the input to each language's
/// key rule).
pub(crate) fn rel_parts(file: &Path, root: &Path) -> Vec<String> {
    file.strip_prefix(root)
        .unwrap_or(file)
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect()
}
