//! Pillar 2: unreferenced-symbol detection (reachability + evidence, tiered).
//!
//! Liveness is modeled from declared entry points (pyproject scripts/plugins +
//! `sensez.toml` globs/base conventions) and usage evidence. Confidence reflects
//! what sensez can actually prove: a symbol in an imported-by-name module that's
//! never referenced is **High**; a module
//! nothing imports is inherently ambiguous (possible undeclared entry point)
//! so its symbols are **Low**. Decorators are classified by *shape*:
//! registration-shaped (`@app.get`) ⇒ live, neutral (`@property`) ⇒ ignored,
//! unknown (`@lru_cache`) ⇒ downgraded to Low. Default scope is top-level
//! functions/classes; imports/methods/variables are opt-in.

mod class_entrypoints;
mod extra;
mod reachability;

use crate::config::model::DeadCode;
use crate::globs::build_globset;
use crate::noze::{ActionLevel, Confidence, DeadCodeFinding, SymbolKind};
use crate::profiles::{registry, DecoratorClass, Language};
use crate::spine::graph::CodebaseGraph;
use crate::spine::parser::ParsedFile;
use globset::GlobSet;
use reachability::{confidence_of, inbound_usage, is_entry_module, skip_symbol};
use std::collections::BTreeMap;
use std::collections::HashSet;

/// Find unreferenced symbols across the codebase.
pub fn detect(cg: &CodebaseGraph, files: &[ParsedFile], config: &DeadCode) -> Vec<DeadCodeFinding> {
    let entry_modules: HashSet<&str> = config.entry_modules.iter().map(String::as_str).collect();
    let rule_sets = RuleSets::from_files(files, config);
    let class_entrypoints =
        class_entrypoints::ClassEntrypoints::from_files(files, &config.entrypoint_bases);
    let mut findings = Vec::new();

    for idx in cg.graph.node_indices() {
        let node = &cg.graph[idx];
        let profile = registry::dead_code_profile(node.language);
        let rules = rule_sets.for_language(node.language);
        if node.is_external || is_entry_module(node, profile, &entry_modules, &rules.entry_globs) {
            continue;
        }
        let inbound = inbound_usage(cg, idx, |source| {
            rule_sets
                .for_language(source.language)
                .test_source_globs
                .is_match(&source.file_path)
        });
        if inbound.star {
            continue; // `from mod import *` consumes everything
        }
        let mut seen = HashSet::new();
        for symbol in &node.declared_public_symbols {
            if !seen.insert(symbol.as_str()) {
                continue;
            }
            let kind = node
                .declared_kinds
                .get(symbol)
                .copied()
                .unwrap_or(SymbolKind::Function);
            if kind == SymbolKind::Variable && !config.unused_variables {
                continue; // module-level variables are opt-in
            }
            let dclass =
                profile.classify_decorator(node.decorators.get(symbol), &rules.entrypoints);
            if matches!(dclass, DecoratorClass::Registration)
                || rules.entrypoint_names.contains(symbol.as_str())
                || class_entrypoints.is_entrypoint(&node.file_path, symbol, kind)
                || skip_symbol(node, profile, symbol, &inbound.used)
            {
                continue;
            }
            let mut confidence = confidence_of(&inbound);
            if matches!(dclass, DecoratorClass::Unknown) {
                confidence = Confidence::Low; // decorated by an unknown wrapper — uncertain
            }
            findings.push(DeadCodeFinding {
                action: ActionLevel::Advisory,
                module: node.module_name.clone(),
                symbol: symbol.clone(),
                kind,
                confidence,
                file: node.file_path.clone(),
                line: node.declared_lines.get(symbol).copied().unwrap_or(0),
                reason: String::new(),
            });
        }
    }

    if config.unused_imports || config.unused_methods {
        let modmap = extra::module_map(cg);
        if config.unused_imports {
            findings.extend(extra::unused_imports(files, &modmap));
        }
        if config.unused_methods {
            findings.extend(extra::unused_methods(files, &modmap, |language, name| {
                rule_sets
                    .for_language(language)
                    .entrypoint_names
                    .contains(name)
            }));
        }
    }
    findings
}

struct RuleSets {
    by_language: BTreeMap<Language, RuleSet>,
}

struct RuleSet {
    entrypoints: HashSet<String>,
    entrypoint_names: HashSet<String>,
    entry_globs: GlobSet,
    test_source_globs: GlobSet,
}

impl RuleSets {
    fn from_files(files: &[ParsedFile], config: &DeadCode) -> Self {
        let languages: HashSet<_> = files.iter().map(|file| file.language).collect();
        let by_language = languages
            .into_iter()
            .map(|language| {
                let defaults = registry::dead_code_profile(language).dead_code_defaults();
                let entrypoints = merged_set(&config.entrypoints, defaults.entrypoints);
                let entrypoint_names =
                    merged_set(&config.entrypoint_names, defaults.entrypoint_names);
                let entry_globs = merged_globset(&config.entry_points, defaults.entry_points);
                let test_source_globs = merged_globset(&[], defaults.test_sources);
                (
                    language,
                    RuleSet {
                        entrypoints,
                        entrypoint_names,
                        entry_globs,
                        test_source_globs,
                    },
                )
            })
            .collect();

        Self { by_language }
    }

    fn for_language(&self, language: Language) -> &RuleSet {
        self.by_language
            .get(&language)
            .expect("dead-code rules exist for every parsed language")
    }
}

fn merged_set(configured: &[String], defaults: &'static [&'static str]) -> HashSet<String> {
    defaults
        .iter()
        .copied()
        .map(str::to_string)
        .chain(configured.iter().cloned())
        .collect()
}

fn merged_globset(configured: &[String], defaults: &'static [&'static str]) -> GlobSet {
    let globs: Vec<String> = defaults
        .iter()
        .copied()
        .map(str::to_string)
        .chain(configured.iter().cloned())
        .collect();
    build_globset(&globs)
}

#[cfg(test)]
mod test_sources;
#[cfg(test)]
mod tests;
