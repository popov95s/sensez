use super::facts::{DynamicFacts, FileFacts};
#[cfg(feature = "lang-javascript")]
use super::javascript;
#[cfg(feature = "lang-python")]
use super::python;
#[cfg(any(feature = "lang-python", feature = "lang-javascript"))]
use crate::spine::ir::Language;
use rayon::prelude::*;
use std::path::{Path, PathBuf};

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
    match crate::reflexez::changes::source_language(path)? {
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

pub fn may_import(path: &Path) -> bool {
    let import_words = crate::profiles::registry::parse_for_path(path)
        .map_or(&[][..], |profile| profile.info().import_words);
    std::fs::read(path).map_or(true, |source| {
        might_contain_dynamic_import(&source)
            || import_words
                .iter()
                .any(|word| contains(&source, word.as_bytes()))
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
