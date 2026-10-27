//! Pillar 2: unreferenced-symbol detection (reachability + evidence, tiered).
//!
//! Liveness is a walk from production roots (manifest entries, entry globs,
//! tool configs, runtime-loaded directories) and test roots (test sources).
//! Importers count as usage when production reaches them; a symbol only tests
//! ask for is reported as `test_only`. Reachability is a module property: a
//! module no root reaches, or only tests reach, is reported once as kind
//! `module`. An unused name is reported where it is defined; a barrel
//! re-export only when nothing defines or uses the name elsewhere. Confidence
//! is High unless evidence suggests hidden use (default export, opaque or
//! type-namespace importer, tool config, test-only, unreachable) → Medium.
//! Decorators are classified by *shape*: registration-shaped (`@app.get`) ⇒
//! live, neutral (`@property`) ⇒ ignored, unknown (`@lru_cache`) ⇒ not reported.

mod class_entrypoints;
mod confidence;
mod demand;
mod extra;
mod loaded_dirs;
mod manifest_callees;
mod modules;
mod reach;
mod reachability;
mod roots;

use crate::config::model::DeadCode;
use crate::globs::{compile_validated, matches_any};
use crate::profiles::{registry, EntryCandidate};
use crate::report::DeadCodeFinding;
use crate::spine::graph::{CodebaseGraph, ModuleNode};
use crate::spine::ir::Language;
use crate::spine::parser::ParsedFile;
use demand::TestImporters;
use globset::GlobSet;
use std::collections::BTreeMap;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Find unreferenced symbols across the codebase.
pub fn detect(cg: &CodebaseGraph, files: &[ParsedFile], config: &DeadCode) -> Vec<DeadCodeFinding> {
    let rule_sets = RuleSets::from_files(files, config);
    let scan = modules::ModuleScan::new(cg, files, config, &rule_sets);
    let mut findings: Vec<DeadCodeFinding> = cg
        .graph
        .node_indices()
        .flat_map(|idx| scan.findings(idx))
        .collect();
    findings.extend(extra_findings(cg, files, config, &rule_sets));
    findings
}

/// Unused imports, methods and properties, each behind its own knob.
fn extra_findings(
    cg: &CodebaseGraph,
    files: &[ParsedFile],
    config: &DeadCode,
    rule_sets: &RuleSets,
) -> Vec<DeadCodeFinding> {
    let mut findings = Vec::new();
    if !(config.unused_imports || config.unused_methods || config.unused_properties) {
        return findings;
    }
    let modmap = extra::module_map(cg);
    let report_files: Vec<_> = files
        .iter()
        .filter(|file| !rule_sets.is_ignored_source(file.language, &file.path))
        .collect();
    let usage_files: Vec<_> = files
        .iter()
        .filter(|file| !rule_sets.is_test_source(file.language, &file.path))
        .collect();
    if config.unused_imports {
        findings.extend(extra::unused_imports(report_files.iter().copied(), &modmap));
    }
    if config.unused_methods {
        let member_files =
            extra::MemberFiles::new(report_files.iter().copied(), usage_files.iter().copied());
        findings.extend(extra::unused_methods(
            &member_files,
            &modmap,
            |language, name| {
                rule_sets
                    .for_language(language)
                    .is_some_and(|rules| rules.is_method_entrypoint_name(name))
            },
        ));
    }
    if config.unused_properties {
        findings.extend(extra::unused_properties(
            report_files.iter().copied(),
            &modmap,
        ));
    }
    findings
}

struct RuleSets {
    by_language: BTreeMap<Language, RuleSet>,
}

struct RuleSet {
    entrypoints: HashSet<String>,
    entrypoint_names: Vec<String>,
    method_entrypoint_names: Vec<String>,
    entry_globs: GlobSet,
    config_globs: GlobSet,
    test_source_globs: GlobSet,
    test_support_globs: GlobSet,
    manifest_entries: HashSet<PathBuf>,
}

impl RuleSet {
    fn test_importers(&self, path: &Path) -> TestImporters {
        if self.test_support_globs.is_match(path) {
            TestImporters::Count
        } else {
            TestImporters::ReportAsTestOnly
        }
    }

    fn is_entrypoint_name(&self, name: &str) -> bool {
        matches_any(&self.entrypoint_names, name)
    }

    fn is_method_entrypoint_name(&self, name: &str) -> bool {
        self.is_entrypoint_name(name) || matches_any(&self.method_entrypoint_names, name)
    }
}

impl RuleSets {
    fn from_files(files: &[ParsedFile], config: &DeadCode) -> Self {
        let languages: HashSet<_> = files.iter().map(|file| file.language).collect();
        let by_language = languages
            .into_iter()
            .map(|language| {
                let defaults = registry::dead_code_profile(language).dead_code_defaults();
                let entrypoints = merged(
                    &config.registration_decorators,
                    defaults.registration_decorators,
                );
                let entrypoint_names = merged(&config.entrypoint_names, defaults.entrypoint_names);
                let method_entrypoint_names = merged(&[], defaults.method_entrypoint_names);
                let entry_globs = merged_globset(&config.entry_globs, defaults.entry_globs);
                let config_globs = merged_globset(&[], defaults.runner_configs);
                let test_source_globs = merged_globset(&config.test_sources, defaults.test_sources);
                let test_support_globs =
                    merged_globset(&config.test_support, defaults.test_support);
                let manifest_entries = manifest_entries(files, language);
                (
                    language,
                    RuleSet {
                        entrypoints,
                        entrypoint_names,
                        method_entrypoint_names,
                        entry_globs,
                        config_globs,
                        test_source_globs,
                        test_support_globs,
                        manifest_entries,
                    },
                )
            })
            .collect();

        Self { by_language }
    }

    fn for_language(&self, language: Language) -> Option<&RuleSet> {
        self.by_language.get(&language)
    }

    fn is_ignored_source(&self, language: Language, path: &Path) -> bool {
        self.for_language(language).is_some_and(|rules| {
            rules.test_source_globs.is_match(path) || rules.entry_globs.is_match(path)
        })
    }

    fn is_test_source(&self, language: Language, path: &Path) -> bool {
        self.for_language(language)
            .is_some_and(|rules| rules.test_source_globs.is_match(path))
    }

    fn is_config_file(&self, node: &ModuleNode) -> bool {
        self.for_language(node.language)
            .is_some_and(|rules| rules.config_globs.is_match(&node.file_path))
    }
}

fn manifest_entries(files: &[ParsedFile], language: Language) -> HashSet<PathBuf> {
    let paths: Vec<EntryCandidate> = files
        .iter()
        .filter(|file| file.language == language)
        .map(|file| EntryCandidate {
            path: &file.path,
            runs_as_program: file.runs_as_program,
        })
        .collect();
    registry::dead_code_profile(language)
        .manifest_entry_files(&paths)
        .into_iter()
        .collect()
}

fn merged<C: FromIterator<String>>(configured: &[String], defaults: &'static [&'static str]) -> C {
    defaults
        .iter()
        .copied()
        .map(str::to_string)
        .chain(configured.iter().cloned())
        .collect()
}

fn merged_globset(configured: &[String], defaults: &'static [&'static str]) -> GlobSet {
    compile_validated(&merged::<Vec<String>>(configured, defaults))
}

#[cfg(test)]
mod member_tests;
#[cfg(test)]
mod test_sources;
#[cfg(test)]
mod tests;
