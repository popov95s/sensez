//! On-demand dynamic-import extraction. This module is never reached by Noze scans.

#[cfg(feature = "lang-javascript")]
pub(super) mod javascript;
#[cfg(feature = "lang-python")]
mod python;

use crate::spine::ir::{ImportContext, Language};
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct FileFacts {
    pub imports: Vec<ImportContext>,
    /// One entry per glob call; `!`-prefixed patterns exclude matches.
    pub patterns: Vec<Vec<String>>,
    pub unresolved: usize,
}

#[derive(Default)]
pub struct DynamicFacts {
    pub by_file: HashMap<PathBuf, FileFacts>,
    pub unresolved: usize,
}

pub fn scan(files: &[PathBuf]) -> DynamicFacts {
    let found: Vec<_> = files
        .par_iter()
        .filter_map(|path| scan_file(path).map(|facts| (path.clone(), facts)))
        .collect();
    let unresolved = found.iter().map(|(_, facts)| facts.unresolved).sum();
    DynamicFacts {
        by_file: found.into_iter().collect(),
        unresolved,
    }
}

fn scan_file(path: &Path) -> Option<FileFacts> {
    let source = std::fs::read(path).ok()?;
    if !might_contain_dynamic_import(&source) {
        return None;
    }
    match super::changes::source_language(path)? {
        #[cfg(feature = "lang-python")]
        Language::Python => python::scan(&source, module_name(path)),
        #[cfg(feature = "lang-javascript")]
        Language::JavaScript | Language::TypeScript => {
            javascript::scan(path, &source, module_name(path))
        }
        _ => None,
    }
}

fn might_contain_dynamic_import(source: &[u8]) -> bool {
    contains(source, b"import")
        || contains(source, b"require")
        || contains(source, b"import_module")
        || contains(source, b"__import__")
        || contains(source, b"import.meta.glob")
}

/// Whether the file at `path` may import anything; unreadable files may.
pub fn may_import(path: &Path) -> bool {
    std::fs::read(path).map_or(true, |source| {
        might_contain_dynamic_import(&source) || contains(&source, b"from")
    })
}

fn contains(source: &[u8], needle: &[u8]) -> bool {
    source.windows(needle.len()).any(|window| window == needle)
}

fn module_name(path: &Path) -> &str {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
}

pub(super) fn context(target: String, source: &str, line: usize) -> ImportContext {
    ImportContext {
        source_module: source.to_string(),
        target_module: target,
        imported_symbols: Vec::new(),
        bindings: Vec::new(),
        binding_phases: Vec::new(),
        line,
        column: 1,
        phase: crate::spine::ir::ImportPhase::Runtime,
        is_inline: true,
        is_module_decl: false,
        is_reexport: false,
        enclosing_scope: None,
    }
}
