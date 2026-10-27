//! Unreferenced module-level symbols and re-exports, one module at a time.

use super::class_entrypoints::ClassEntrypoints;
use super::confidence::{grade, Evidence, Reached};
use super::demand::{self, Demand, Importers, Inbound, TestImporters};
use super::manifest_callees::ManifestCallees;
use super::reach::{self, Reach};
use super::reachability::{is_asked, skip_symbol};
use super::reexports::{self, Barrel};
use super::roots::{self, Role};
use super::{loaded_dirs, RuleSet, RuleSets};
use crate::config::model::DeadCode;
use crate::profiles::{registry, DeadCodeProfile};
use crate::report::{ActionLevel, DeadCodeFinding};
use crate::spine::graph::{CodebaseGraph, ModuleNode};
use crate::spine::parser::{ParsedFile, SymbolKind};
use petgraph::graph::NodeIndex;
use std::collections::{HashMap, HashSet};

/// Roots, reachability and importer demand shared by every module's findings.
pub(super) struct ModuleScan<'a> {
    cg: &'a CodebaseGraph,
    files: &'a [ParsedFile],
    config: &'a DeadCode,
    rule_sets: &'a RuleSets,
    class_entrypoints: ClassEntrypoints,
    manifest_callees: ManifestCallees,
    roles: HashMap<NodeIndex, Role>,
    reach: Reach,
    inbound: HashMap<NodeIndex, Inbound>,
}

impl<'a> ModuleScan<'a> {
    pub(super) fn new(
        cg: &'a CodebaseGraph,
        files: &'a [ParsedFile],
        config: &'a DeadCode,
        rule_sets: &'a RuleSets,
    ) -> Self {
        let entry_modules: HashSet<&str> =
            config.entry_modules.iter().map(String::as_str).collect();
        let mut roles = roots::roles(cg, rule_sets, &entry_modules);
        loaded_dirs::promote(cg, &mut roles, config.unimported_dir_share);
        let reach = reach::walk(cg, &roles);
        let config_files: HashSet<_> = roles
            .keys()
            .copied()
            .filter(|&idx| rule_sets.is_config_file(&cg.graph[idx]))
            .collect();
        let inbound = demand::inbound(
            cg,
            &Importers {
                roles: &roles,
                reach: &reach,
                config_files: &config_files,
            },
        );
        Self {
            cg,
            files,
            config,
            rule_sets,
            class_entrypoints: ClassEntrypoints::from_files(files, &config.entrypoint_bases),
            manifest_callees: ManifestCallees::new(cg, files, &inbound),
            roles,
            reach,
            inbound,
        }
    }

    /// Findings for one import-only module; empty for roots, tests and externals.
    pub(super) fn findings(&self, idx: NodeIndex) -> Vec<DeadCodeFinding> {
        if self.roles.get(&idx) != Some(&Role::Module) {
            return Vec::new();
        }
        let node = &self.cg.graph[idx];
        let Some(rules) = self.rule_sets.for_language(node.language) else {
            return Vec::new();
        };
        let test_importers = rules.test_importers(&node.file_path);
        let reached = self.reached(idx, test_importers);
        let silent_orphan = !self.config.orphan_modules || test_importers == TestImporters::Count;
        if reached == Reached::Orphan && silent_orphan {
            return Vec::new();
        }
        let Some(file) = node.source_index.and_then(|index| self.files.get(index)) else {
            return Vec::new();
        };
        let no_importers = Inbound::default();
        let inbound = self.inbound.get(&idx).unwrap_or(&no_importers);
        let counted = inbound.counted(test_importers);
        if counted.all {
            return Vec::new();
        }
        let module = Module {
            node,
            file,
            rules,
            profile: registry::dead_code_profile(node.language),
            inbound,
            counted: &counted,
            reached,
        };
        let mut seen = HashSet::new();
        let mut findings: Vec<DeadCodeFinding> = file
            .walked
            .symbols
            .declared
            .iter()
            .filter(|symbol| seen.insert(symbol.as_str()))
            .filter_map(|symbol| self.symbol_finding(&module, symbol))
            .collect();
        let barrel = Barrel {
            idx,
            inbound,
            counted: &counted,
            reached,
            rules,
        };
        findings.extend(reexports::unconsumed(self.cg, &barrel, |origin, name| {
            self.origin_accounts(origin, name)
        }));
        match reached {
            Reached::FromRoot => findings,
            Reached::Orphan | Reached::TestsOnly => whole_module(node, findings),
        }
    }

    fn reached(&self, idx: NodeIndex, test_importers: TestImporters) -> Reached {
        if self.reach.orphans.contains(&idx) {
            Reached::Orphan
        } else if !self.reach.prod.contains(&idx)
            && test_importers == TestImporters::ReportAsTestOnly
        {
            Reached::TestsOnly
        } else {
            Reached::FromRoot
        }
    }

    /// The defining module reports, uses, re-exports or types `name` itself, so its barrel need not.
    fn origin_accounts(&self, origin: NodeIndex, name: &str) -> bool {
        let Some(file) = self.cg.graph[origin]
            .source_index
            .and_then(|index| self.files.get(index))
        else {
            return false;
        };
        let asked = self
            .inbound
            .get(&origin)
            .is_some_and(|inbound| inbound.prod.contains(name) || inbound.test.contains(name));
        asked
            || file.walked.symbols.type_names.contains(name)
            || file.walked.symbols.declared_kinds.contains_key(name)
            || file
                .walked
                .usage
                .name_counts
                .get(name)
                .copied()
                .unwrap_or(0)
                > 1
            || reexports::takes_from_elsewhere(self.cg, origin, name)
    }

    fn symbol_finding(&self, module: &Module, symbol: &String) -> Option<DeadCodeFinding> {
        let symbols = &module.file.walked.symbols;
        let kind = symbols
            .declared_kinds
            .get(symbol)
            .copied()
            .unwrap_or(SymbolKind::Function);
        if kind == SymbolKind::Variable && !self.config.unused_variables {
            return None;
        }
        let decorator = module
            .profile
            .classify_decorator(symbols.decorators.get(symbol), &module.rules.entrypoints);
        let live = decorator.is_registration()
            || (module.inbound.type_star
                && self
                    .manifest_callees
                    .registers(symbols.decorators.get(symbol)))
            || module.rules.is_entrypoint_name(symbol)
            || self
                .class_entrypoints
                .is_entrypoint(&module.node.file_path, symbol, kind)
            || skip_symbol(module.file, module.profile, symbol, module.counted);
        if live || decorator.is_unknown() {
            return None;
        }
        let grade = grade(&Evidence {
            inbound: module.inbound,
            reached: module.reached,
            is_default_export: symbols.default_export.as_deref() == Some(symbol.as_str()),
            test_only: is_asked(module.file, symbol, &module.inbound.test),
        });
        Some(DeadCodeFinding {
            action: ActionLevel::Advisory,
            module: module.node.module_name.clone(),
            symbol: symbol.clone(),
            kind,
            confidence: grade.confidence,
            file: module.node.file_path.clone(),
            line: symbols.declared_lines.get(symbol).copied().unwrap_or(0),
            reason: grade.reason_text(),
        })
    }
}

/// One module's facts while its symbols are graded.
struct Module<'a> {
    node: &'a ModuleNode,
    file: &'a ParsedFile,
    rules: &'a RuleSet,
    profile: &'static dyn DeadCodeProfile,
    inbound: &'a Inbound,
    counted: &'a Demand,
    reached: Reached,
}

/// One finding for a module no production root reaches, naming its unused symbols.
fn whole_module(node: &ModuleNode, mut findings: Vec<DeadCodeFinding>) -> Vec<DeadCodeFinding> {
    findings.sort_by_key(|finding| (finding.line == 0, finding.line));
    let symbol = findings
        .iter()
        .map(|finding| finding.symbol.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let Some(first) = findings.into_iter().next() else {
        return Vec::new();
    };
    vec![DeadCodeFinding {
        module: node.module_name.clone(),
        symbol,
        kind: SymbolKind::Module,
        ..first
    }]
}
