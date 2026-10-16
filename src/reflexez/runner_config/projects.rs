use super::glob_set;
use super::value::Value;
use crate::reflexez::patterns::normalize_path;
use std::path::{Path, PathBuf};

/// One entry of a `projects`/workspace array.
pub enum Entry {
    Inline(Value),
    Path(PathBuf),
}

/// Entries of a projects array, plus reasons for entries that could not be read.
pub fn entries(value: &Value, base: &Path) -> (Vec<Entry>, Vec<String>) {
    let items = match value {
        Value::Array(items) => items.clone(),
        other => vec![other.clone()],
    };
    let mut entries = Vec::new();
    let mut unreadable = Vec::new();
    let mut globs = Vec::new();
    let mut negations = Vec::new();
    for item in items {
        match item {
            Value::Object(_) => entries.push(Entry::Inline(item)),
            Value::Str(path) => match path.strip_prefix('!') {
                Some(negated) => negations.push(negated.to_string()),
                None if path.contains(['*', '?', '{', '[']) => globs.push(path),
                None => entries.push(Entry::Path(normalize_path(&base.join(path)))),
            },
            Value::Opaque(reason) => unreadable.push(reason),
            _ => unreadable.push("project entry is not a string or an object".into()),
        }
    }
    match expand(base, &globs, &negations) {
        Ok(paths) => entries.extend(paths.into_iter().map(Entry::Path)),
        Err(reason) => unreadable.push(reason),
    }
    (entries, unreadable)
}

fn expand(base: &Path, globs: &[String], negations: &[String]) -> Result<Vec<PathBuf>, String> {
    if globs.is_empty() {
        return Ok(Vec::new());
    }
    let trimmed = |patterns: &[String]| -> Vec<String> {
        patterns
            .iter()
            .map(|pattern| pattern.trim_end_matches('/').to_string())
            .collect()
    };
    let include = glob_set(&trimmed(globs))?;
    let exclude = glob_set(&trimmed(negations))?;
    let walker = ignore::WalkBuilder::new(base)
        .filter_entry(|entry| entry.file_name() != "node_modules")
        .build();
    Ok(walker
        .filter_map(Result::ok)
        .map(ignore::DirEntry::into_path)
        .filter(|path| {
            let Ok(relative) = path.strip_prefix(base) else {
                return false;
            };
            let relative = relative.to_string_lossy().replace('\\', "/");
            !relative.is_empty()
                && include.is_match(&relative)
                && !exclude.is_match(&relative)
                && (path.is_dir() || is_config_file(path))
        })
        .collect())
}

fn is_config_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.contains(".config.") || name.contains(".workspace."))
}
