//! Well-known utility tables and declared dependencies, supplied per language.

use crate::spine::ir::{ProviderOrigin, UtilityShape};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Where a utility's provider lives.
#[derive(Debug, Clone, Copy)]
pub enum ProviderSource {
    StandardLibrary {
        module: &'static str,
    },
    /// `name` as the manifest spells it, `module` as an import spells it.
    Package {
        name: &'static str,
        module: &'static str,
    },
}

/// One function that implements a utility.
#[derive(Debug, Clone, Copy)]
pub struct Provider {
    pub source: ProviderSource,
    pub function: &'static str,
}

/// A utility people re-write by hand, and the functions that already provide it.
#[derive(Debug, Clone, Copy)]
pub struct KnownUtility {
    /// Canonical name first, then common aliases.
    pub names: &'static [&'static str],
    pub shape: UtilityShape,
    pub providers: &'static [Provider],
}

pub const fn shape(min_arity: usize, max_arity: usize, max_lines: usize) -> UtilityShape {
    UtilityShape {
        min_arity,
        max_arity,
        max_lines,
    }
}

/// Which manifest sections count for a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyScope {
    Production,
    Development,
}

/// Packages the manifests above a directory declare, each with the manifest file naming it.
#[derive(Debug, Clone, Default)]
pub struct DeclaredPackages {
    pub production: HashMap<String, String>,
    pub development: HashMap<String, String>,
}

impl DeclaredPackages {
    /// Why `package` is available to code in `scope`, if it is.
    pub fn origin(&self, package: &str, scope: DependencyScope) -> Option<ProviderOrigin> {
        let development = match scope {
            DependencyScope::Production => None,
            DependencyScope::Development => self.development.get(package),
        };
        self.production
            .get(package)
            .or(development)
            .map(|manifest| ProviderOrigin::Manifest(manifest.clone()))
    }

    /// Record `package` from `manifest` without overriding a nearer declaration.
    pub fn declare(&mut self, scope: DependencyScope, package: String, manifest: &str) {
        let section = match scope {
            DependencyScope::Production => &mut self.production,
            DependencyScope::Development => &mut self.development,
        };
        section
            .entry(package)
            .or_insert_with(|| manifest.to_string());
    }
}

pub trait DependencyProfile: Send + Sync {
    /// Utilities this language's ecosystem already provides.
    fn known_utilities(&self) -> &'static [KnownUtility] {
        &[]
    }
    /// Packages the manifests at or above `dir` declare.
    fn declared_packages(&self, _dir: &Path) -> DeclaredPackages {
        DeclaredPackages::default()
    }
}

/// `dir` and its ancestors up to and including the repository root.
pub fn manifest_dirs(dir: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    for ancestor in dir.ancestors() {
        dirs.push(ancestor.to_path_buf());
        if ancestor.join(".git").exists() {
            break;
        }
    }
    dirs
}
