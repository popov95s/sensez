//! TypeScript `tsconfig` path-alias resolution.
//!
//! This profile module intentionally owns the TypeScript-specific
//! `compilerOptions.paths` behavior. The shared graph receives only the
//! resulting, language-neutral module target.

use super::tsconfig::read_tsconfig;
use super::type_imports::{self, TypeOnlyImports};
use crate::profiles::javascript::{resolve, roots};
use crate::profiles::ImportResolver;
use crate::spine::ir::{ImportContext, ImportPhase};
use serde_json::Value;
use std::collections::hash_map::Entry as CacheEntry;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// TypeScript resolver state scoped to one graph build.
///
/// A scan can include multiple package roots, each with its own alias table.
/// Both valid and empty tables are retained so a missing or malformed config is
/// attempted once per root, rather than once per import.
#[derive(Default)]
pub(super) struct ResolutionCache {
    aliases_by_root: HashMap<PathBuf, Paths>,
    type_only_imports_by_dir: HashMap<PathBuf, TypeOnlyImports>,
    #[cfg(test)]
    loads: usize,
}

impl ResolutionCache {
    pub(super) fn aliases(&mut self, root: &Path) -> &Paths {
        match self.aliases_by_root.entry(root.to_path_buf()) {
            CacheEntry::Occupied(entry) => entry.into_mut(),
            CacheEntry::Vacant(entry) => {
                #[cfg(test)]
                {
                    self.loads += 1;
                }
                entry.insert(load_aliases(root))
            }
        }
    }

    /// Emit mode of inline type-only imports under the nearest `tsconfig.json` above `dir`.
    pub(super) fn type_only_imports(&mut self, dir: &Path) -> TypeOnlyImports {
        if let Some(&mode) = self.type_only_imports_by_dir.get(dir) {
            return mode;
        }
        let config = dir.join("tsconfig.json");
        let mode = if config.is_file() {
            type_imports::load(&config)
        } else if let Some(parent) = dir.parent() {
            self.type_only_imports(parent)
        } else {
            TypeOnlyImports::Erased
        };
        self.type_only_imports_by_dir
            .insert(dir.to_path_buf(), mode);
        mode
    }

    #[cfg(test)]
    pub(super) fn load_count(&self) -> usize {
        self.loads
    }
}

impl ImportResolver for ResolutionCache {
    fn resolve_target(
        &mut self,
        import: &ImportContext,
        _importer_package: &str,
        file: &Path,
        root: &Path,
    ) -> String {
        self.aliases(root)
            .resolve(&import.target_module)
            .unwrap_or_else(|| resolve::resolve_target(import, file, root))
    }

    fn edge_phase(&mut self, import: &ImportContext, importer_file: &Path) -> ImportPhase {
        super::edge_phase::edge_phase(self, import, importer_file)
    }

    fn alias_path(&mut self, root: &Path, specifier: &str) -> Option<PathBuf> {
        self.aliases(root).resolve_path(specifier)
    }
}

pub(super) struct Paths {
    root: PathBuf,
    base_url: PathBuf,
    entries: Vec<Entry>,
}

struct Entry {
    pattern: String,
    targets: Vec<String>,
}

impl Paths {
    pub(super) fn resolve(&self, specifier: &str) -> Option<String> {
        self.resolve_path(specifier)
            .map(|path| roots::module_name(&path, &self.root))
    }

    fn resolve_path(&self, specifier: &str) -> Option<PathBuf> {
        self.entries
            .iter()
            .filter_map(|entry| match_pattern(&entry.pattern, specifier).map(|part| (entry, part)))
            .max_by_key(|(entry, _)| entry.pattern.len())
            .and_then(|(entry, part)| entry.targets.first().map(|target| (target, part)))
            .map(|(target, part)| self.base_url.join(target.replace('*', part)))
    }
}

fn load_aliases(root: &Path) -> Paths {
    let Some(value) = read_tsconfig(&root.join("tsconfig.json")) else {
        return Paths {
            root: root.to_path_buf(),
            base_url: root.to_path_buf(),
            entries: Vec::new(),
        };
    };
    let options = value.get("compilerOptions").and_then(Value::as_object);
    let base_url = options
        .and_then(|options| options.get("baseUrl"))
        .and_then(Value::as_str)
        .map(|base| root.join(base))
        .unwrap_or_else(|| root.to_path_buf());
    let entries = options
        .and_then(|options| options.get("paths"))
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|paths| paths.iter())
        .filter_map(|(pattern, targets)| {
            let targets: Vec<_> = targets
                .as_array()?
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect();
            (!targets.is_empty()).then(|| Entry {
                pattern: pattern.clone(),
                targets,
            })
        })
        .collect();
    Paths {
        root: root.to_path_buf(),
        base_url,
        entries,
    }
}

fn match_pattern<'a>(pattern: &'a str, specifier: &'a str) -> Option<&'a str> {
    match pattern.split_once('*') {
        Some((prefix, suffix)) if specifier.starts_with(prefix) && specifier.ends_with(suffix) => {
            let end = specifier.len().checked_sub(suffix.len())?;
            Some(&specifier[prefix.len()..end])
        }
        Some(_) => None,
        None if pattern == specifier => Some(""),
        None => None,
    }
}
