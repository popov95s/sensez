use std::path::{Path, PathBuf};

pub const SCRIPT_EXTENSIONS: [&str; 6] = ["ts", "mts", "cts", "js", "mjs", "cjs"];
pub const MODULE_EXTENSIONS: [&str; 7] = ["ts", "mts", "cts", "js", "mjs", "cjs", "json"];
pub const MAX_PROJECT_DEPTH: usize = 4;

/// First `<stem>.<ext>` file in `dir`, trying stems in order.
pub fn first(dir: &Path, stems: &[&str], extensions: &[&str]) -> Option<PathBuf> {
    stems
        .iter()
        .flat_map(|stem| {
            extensions
                .iter()
                .map(move |ext| dir.join(format!("{stem}.{ext}")))
        })
        .find(|path| path.is_file())
}
