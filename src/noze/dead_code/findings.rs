use super::rule_sets::RuleSets;
use super::{extra, modules};
use crate::config::model::DeadCode;
use crate::report::DeadCodeFinding;
use crate::spine::graph::CodebaseGraph;
use crate::spine::parser::ParsedFile;

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
