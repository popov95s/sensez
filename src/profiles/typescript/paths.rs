//! TypeScript `tsconfig` path-alias resolution.
//!
//! This profile module intentionally owns the TypeScript-specific
//! `compilerOptions.paths` behavior. The shared graph receives only the
//! resulting, language-neutral module target.

use super::compiler_options::CompilerOptions;
use super::type_imports::{self, TypeOnlyImports};
use crate::profiles::javascript::{resolve, roots};
use crate::profiles::ImportResolver;
use crate::spine::ir::{ImportContext, ImportPhase};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// TypeScript resolver state scoped to one graph build, keyed by the nearest `tsconfig.json`.
#[derive(Default)]
pub(super) struct ResolutionCache {
    config_by_dir: HashMap<PathBuf, Option<PathBuf>>,
    aliases_by_config: HashMap<PathBuf, Paths>,
    type_only_imports_by_config: HashMap<PathBuf, TypeOnlyImports>,
    #[cfg(test)]
    loads: usize,
}

impl ResolutionCache {
    /// Aliases of the nearest `tsconfig.json` at or above `dir`.
    pub(super) fn aliases(&mut self, dir: &Path) -> Option<&Paths> {
        let config = self.nearest_config(dir)?;
        if !self.aliases_by_config.contains_key(&config) {
            #[cfg(test)]
            {
                self.loads += 1;
            }
            let paths = Paths::load(&config);
            self.aliases_by_config.insert(config.clone(), paths);
        }
        self.aliases_by_config.get(&config)
    }

    /// Emit mode of inline type-only imports under the nearest `tsconfig.json` above `dir`.
    pub(super) fn type_only_imports(&mut self, dir: &Path) -> TypeOnlyImports {
        let Some(config) = self.nearest_config(dir) else {
            return TypeOnlyImports::Erased;
        };
        *self
            .type_only_imports_by_config
            .entry(config)
            .or_insert_with_key(|config| type_imports::load(config))
    }

    fn nearest_config(&mut self, dir: &Path) -> Option<PathBuf> {
        if let Some(config) = self.config_by_dir.get(dir) {
            return config.clone();
        }
        let candidate = dir.join("tsconfig.json");
        let config = if candidate.is_file() {
            Some(candidate)
        } else {
            dir.parent().and_then(|parent| self.nearest_config(parent))
        };
        self.config_by_dir.insert(dir.to_path_buf(), config.clone());
        config
    }

    fn file_aliases(&mut self, importer_file: &Path) -> Option<&Paths> {
        self.aliases(importer_file.parent().unwrap_or(Path::new("")))
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
        self.file_aliases(file)
            .and_then(|paths| paths.targets(&import.target_module).into_iter().next())
            .map(|path| roots::module_name(&path, root))
            .unwrap_or_else(|| resolve::resolve_target(import, file, root))
    }

    fn target_path_keys(&mut self, import: &ImportContext, importer_file: &Path) -> Vec<PathBuf> {
        let mut keys: Vec<_> = self
            .file_aliases(importer_file)
            .map(|paths| paths.targets(&import.target_module))
            .unwrap_or_default()
            .iter()
            .map(|path| roots::path_key(path))
            .collect();
        keys.extend(resolve::relative_path_key(import, importer_file));
        keys
    }

    fn edge_phase(&mut self, import: &ImportContext, importer_file: &Path) -> ImportPhase {
        super::edge_phase::edge_phase(self, import, importer_file)
    }

    fn alias_path(&mut self, importer_file: &Path, specifier: &str) -> Option<PathBuf> {
        self.file_aliases(importer_file)?
            .targets(specifier)
            .into_iter()
            .next()
    }
}

pub(super) struct Paths {
    base: PathBuf,
    entries: Vec<Entry>,
}

struct Entry {
    pattern: String,
    targets: Vec<String>,
}

impl Paths {
    /// Every target of the longest matching pattern, in declaration order.
    pub(super) fn targets(&self, specifier: &str) -> Vec<PathBuf> {
        self.entries
            .iter()
            .filter_map(|entry| match_pattern(&entry.pattern, specifier).map(|part| (entry, part)))
            .max_by_key(|(entry, _)| entry.pattern.len())
            .map(|(entry, part)| {
                entry
                    .targets
                    .iter()
                    .map(|target| self.base.join(target.replace('*', part)))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn load(config: &Path) -> Self {
        let options = CompilerOptions::load(config);
        let base = options
            .paths_base()
            .unwrap_or_else(|| config.parent().unwrap_or(Path::new("")).to_path_buf());
        let entries = options
            .values
            .get("paths")
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
        Self { base, entries }
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
