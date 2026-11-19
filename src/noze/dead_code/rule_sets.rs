use super::demand::TestImporters;
use crate::config::model::DeadCode;
use crate::globs::{compile_validated, matches_any};
use crate::profiles::{registry, EntryCandidate};
use crate::spine::graph::ModuleNode;
use crate::spine::ir::Language;
use crate::spine::parser::ParsedFile;
use globset::GlobSet;
use std::collections::BTreeMap;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub(super) struct RuleSets {
    pub(super) by_language: BTreeMap<Language, RuleSet>,
}

pub(super) struct RuleSet {
    pub(super) entrypoints: HashSet<String>,
    pub(super) entrypoint_names: Vec<String>,
    pub(super) method_entrypoint_names: Vec<String>,
    pub(super) entry_globs: GlobSet,
    pub(super) config_globs: GlobSet,
    pub(super) test_source_globs: GlobSet,
    pub(super) test_support_globs: GlobSet,
    pub(super) manifest_entries: HashSet<PathBuf>,
}

impl RuleSet {
    pub(super) fn test_importers(&self, path: &Path) -> TestImporters {
        if self.test_support_globs.is_match(path) {
            TestImporters::Count
        } else {
            TestImporters::ReportAsTestOnly
        }
    }

    pub(super) fn is_entrypoint_name(&self, name: &str) -> bool {
        matches_any(&self.entrypoint_names, name)
    }

    pub(super) fn is_method_entrypoint_name(&self, name: &str) -> bool {
        self.is_entrypoint_name(name) || matches_any(&self.method_entrypoint_names, name)
    }
}

impl RuleSets {
    pub(super) fn from_files(files: &[ParsedFile], config: &DeadCode) -> Self {
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

    pub(super) fn for_language(&self, language: Language) -> Option<&RuleSet> {
        self.by_language.get(&language)
    }

    pub(super) fn is_ignored_source(&self, language: Language, path: &Path) -> bool {
        self.for_language(language).is_some_and(|rules| {
            rules.test_source_globs.is_match(path) || rules.entry_globs.is_match(path)
        })
    }

    pub(super) fn is_test_source(&self, language: Language, path: &Path) -> bool {
        self.for_language(language)
            .is_some_and(|rules| rules.test_source_globs.is_match(path))
    }

    pub(super) fn is_config_file(&self, node: &ModuleNode) -> bool {
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
