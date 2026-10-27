//! Small module-level functions that re-implement a utility the project already depends on.

use super::make;
use crate::config::smells::SmellConfig;
use crate::profiles::dependencies::{DeclaredPackages, DependencyScope};
use crate::profiles::registry;
use crate::profiles::utility_lookalikes::{lookalikes, FileDependencies};
use crate::report::{ActionLevel, Severity, SmellFinding, SmellKind};
use crate::spine::ir::{Language, ProviderOrigin, UtilityLookalike, UtilityProvider};
use crate::spine::parser::ParsedFile;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const KIND: SmellKind = SmellKind::DependencyReimplementation;
/// Copies of one utility across the project that raise the finding to advisory.
const ADVISORY_COPIES: usize = 3;

type ManifestCache = HashMap<(Language, PathBuf), DeclaredPackages>;
type CopyKey = (Language, &'static str);

pub fn detect(files: &[&ParsedFile], cfg: &SmellConfig) -> Vec<SmellFinding> {
    let mut manifests = ManifestCache::new();
    let copies: Vec<(&ParsedFile, UtilityLookalike)> = files
        .iter()
        .flat_map(|file| {
            copies_in(file, cfg, &mut manifests)
                .into_iter()
                .map(move |copy| (*file, copy))
        })
        .collect();
    let mut counts: HashMap<CopyKey, usize> = HashMap::new();
    for (file, copy) in &copies {
        *counts.entry(copy_key(file, copy)).or_default() += 1;
    }
    copies
        .iter()
        .map(|(file, copy)| finding(file, copy, counts[&copy_key(file, copy)]))
        .collect()
}

fn copies_in(
    file: &ParsedFile,
    cfg: &SmellConfig,
    manifests: &mut ManifestCache,
) -> Vec<UtilityLookalike> {
    let settings = cfg.for_language(file.language);
    if !settings.is_enabled(KIND) {
        return Vec::new();
    }
    let profile = registry::dependency_profile(file.language);
    let dir = file.path.parent().unwrap_or(Path::new("."));
    let declared = manifests
        .entry((file.language, dir.to_path_buf()))
        .or_insert_with(|| profile.declared_packages(dir));
    let dependencies = FileDependencies {
        declared,
        scope: scope_of(file),
        configured: &settings.dependency_reimplementation_providers,
    };
    let units = &file.walked.units.functions;
    let symbols = &file.walked.symbols;
    lookalikes(profile.known_utilities(), units, symbols, &dependencies)
        .into_iter()
        .filter(fits_shape)
        .collect()
}

fn scope_of(file: &ParsedFile) -> DependencyScope {
    match registry::is_test_or_support_source(file.language, &file.path) {
        true => DependencyScope::Development,
        false => DependencyScope::Production,
    }
}

/// The name alone never fires: arity and body size must match the utility's usual copy.
fn fits_shape(copy: &UtilityLookalike) -> bool {
    let shape = copy.shape;
    (shape.min_arity..=shape.max_arity).contains(&copy.arity) && copy.lines <= shape.max_lines
}

fn copy_key(file: &ParsedFile, copy: &UtilityLookalike) -> CopyKey {
    (registry::clone_family(file.language), copy.utility)
}

fn finding(file: &ParsedFile, copy: &UtilityLookalike, copies: usize) -> SmellFinding {
    let mut message = format!(
        "`{}` re-implements {}",
        copy.name,
        suggestion(&copy.providers[0])
    );
    if copies >= ADVISORY_COPIES {
        message.push_str(&format!("; {copies} copies across the project"));
    }
    let mut finding = make(
        KIND,
        message,
        &file.path,
        copy.line,
        &copy.name,
        Severity::Info,
        copies as u32,
        ADVISORY_COPIES as u32,
    );
    finding.end_line = copy.line + copy.lines - 1;
    finding.action = match copies >= ADVISORY_COPIES {
        true => ActionLevel::Advisory,
        false => ActionLevel::Info,
    };
    finding
}

fn suggestion(provider: &UtilityProvider) -> String {
    let UtilityProvider {
        module,
        function,
        origin,
    } = provider;
    match origin {
        ProviderOrigin::StandardLibrary => {
            format!("`{module}.{function}` from the standard library")
        }
        ProviderOrigin::Manifest(manifest) => {
            format!("`{function}` from {module}, which {manifest} already lists")
        }
        ProviderOrigin::Configured => {
            format!("`{function}` from {module}, listed in dependency_reimplementation_providers")
        }
    }
}
