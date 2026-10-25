use super::assets::asset_nodes;
use super::changes;
use super::discovery::{self, TestFile};
use super::dynamic::{self, DynamicFacts};
use super::model::{ImpactPlan, PlanReason, RunMode, Selection};
use super::reach::ChangeReach;
use super::selector_dynamic::{self, Reach};
use super::setup_trigger::setup_fallback;
use crate::cli::spec::ReflexezArgs;
use crate::report::ScanIssue;
use crate::spine::graph::CodebaseGraph;
use crate::spine::parser::ParsedFile;
use anyhow::{bail, Result};
use petgraph::graph::NodeIndex;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub fn plan(root: &Path, args: &ReflexezArgs) -> Result<ImpactPlan> {
    let started = std::time::Instant::now();
    let scope = changes::resolve(root, args.base.as_deref(), args.staged, &args.changed_files)?;
    let project = discovery::discover(&scope.repository, args.runner)?;
    if project.tests.is_empty() {
        bail!("no pytest, Vitest, or Jest test files were discovered");
    }
    let dynamic = dynamic::scan(&project.sources);
    let mut parsed = crate::spine::parser::parse_files(&project.sources);
    add_dynamic_imports(&mut parsed.files, &dynamic);
    let (config, settings_warnings) =
        crate::config::model::Config::load_for_scan(&scope.repository);
    let graph = crate::spine::graph::build(&parsed.files, &config.roots);
    let mut fallback = scope.fallback_reasons;
    let hides_tests = |issue: &ScanIssue| {
        issue
            .file
            .as_deref()
            .is_none_or(|file| !crate::profiles::registry::is_declaration_only(file))
    };
    if parsed.issues.iter().any(hides_tests) {
        fallback.push("incomplete parsing could hide an affected test".into());
    }
    if args.full {
        fallback.push("full-suite execution was explicitly requested".into());
    }
    fallback.extend(setup_fallback(
        &graph,
        &project.setup_files,
        &scope.files,
        &scope.repository,
    ));
    let tests_by_path = tests_by_path(&project.tests);
    let nodes_by_path = nodes_by_path(&graph);
    let opaque_import_files = opaque_files(&dynamic);
    let reached = ChangeReach {
        tests: &tests_by_path,
        nodes: &nodes_by_path,
        reverse: &selector_dynamic::reverse_edges(
            &graph,
            &dynamic,
            &nodes_by_path,
            &scope.repository,
        ),
        assets: &asset_nodes(&graph, &config.roots),
        opaque_files: &opaque_import_files,
    }
    .reach(&scope.files);
    let mut selected = reached.selected;
    if reached.missing_source {
        fallback.push("a changed source file is unavailable for graph analysis".into());
    }
    if args.strict_dynamic {
        fallback.extend(strict_reasons(
            dynamic.unresolved,
            project.config_warnings.len(),
            &reached.unmapped,
        ));
    }
    fallback.sort();
    fallback.dedup();
    let full_suite = !fallback.is_empty();
    if full_suite {
        selected = project
            .tests
            .iter()
            .map(|test| {
                (
                    test.file.clone(),
                    Reach {
                        distance: 0,
                        dynamic: false,
                    },
                )
            })
            .collect();
    }
    let mode = if full_suite {
        RunMode::Full
    } else {
        RunMode::Narrowed
    };
    let selected_paths: HashSet<_> = selected.keys().cloned().collect();
    let selections = selections(selected, &tests_by_path, full_suite, args.full);
    let runners = super::runners::plans(
        &scope.repository,
        &project.runners,
        &project.tests,
        &selected_paths,
        mode,
    );
    Ok(ImpactPlan {
        repository: scope.repository,
        changed_files: scope.files,
        discovered_tests: project.tests.len(),
        selected: selections,
        runners,
        full_suite,
        fallback_reasons: fallback,
        config_warnings: project.config_warnings,
        settings_warnings: settings_warnings
            .into_iter()
            .map(|issue| issue.message)
            .collect(),
        unresolved_dynamic_imports: dynamic.unresolved,
        opaque_import_files,
        unmapped_changes: reached.unmapped,
        selection_ms: started.elapsed().as_millis(),
    })
}

fn strict_reasons(unresolved: usize, config_warnings: usize, unmapped: &[PathBuf]) -> Vec<String> {
    let mut reasons = Vec::new();
    if unresolved > 0 {
        reasons.push(format!(
            "strict dynamic safety found {unresolved} unresolved computed import(s)"
        ));
    }
    if config_warnings > 0 {
        reasons.push(format!(
            "strict dynamic safety found {config_warnings} runner config key(s) that are not statically readable"
        ));
    }
    if unmapped.iter().any(|path| changes::is_source(path)) {
        reasons.push(
            "strict dynamic safety found a changed source file outside the import graph".into(),
        );
    }
    reasons
}

fn opaque_files(dynamic: &DynamicFacts) -> Vec<PathBuf> {
    let mut files: Vec<_> = dynamic
        .by_file
        .iter()
        .filter(|(_, facts)| facts.unresolved > 0)
        .map(|(path, _)| path.clone())
        .collect();
    files.sort();
    files
}

fn add_dynamic_imports(files: &mut [ParsedFile], dynamic: &DynamicFacts) {
    for file in files {
        if let Some(facts) = dynamic.by_file.get(&file.path) {
            Arc::make_mut(&mut file.walked)
                .symbols
                .imports
                .extend(facts.imports.iter().cloned());
        }
    }
}

fn tests_by_path(tests: &[TestFile]) -> HashMap<PathBuf, &TestFile> {
    tests.iter().map(|test| (test.file.clone(), test)).collect()
}

fn nodes_by_path(graph: &CodebaseGraph) -> HashMap<PathBuf, NodeIndex> {
    graph
        .graph
        .node_indices()
        .filter_map(|index| {
            let node = &graph.graph[index];
            (!node.is_external).then(|| (node.file_path.clone(), index))
        })
        .collect()
}

fn selections(
    reached: HashMap<PathBuf, Reach>,
    tests: &HashMap<PathBuf, &TestFile>,
    full_suite: bool,
    requested: bool,
) -> Vec<Selection> {
    let mut selected: Vec<_> = reached
        .into_iter()
        .filter_map(|(file, reach)| {
            let test = tests.get(&file)?;
            let reason = if full_suite {
                if requested {
                    PlanReason::FullRequested
                } else {
                    PlanReason::SafetyFallback
                }
            } else if reach.distance == 0 {
                PlanReason::ChangedTest
            } else if reach.dynamic {
                PlanReason::DynamicImport
            } else if reach.distance == 1 {
                PlanReason::DirectDependency
            } else {
                PlanReason::TransitiveDependency
            };
            Some(Selection {
                file,
                runner: test.runner,
                reason,
                distance: reach.distance,
            })
        })
        .collect();
    selected.sort_by(|left, right| left.file.cmp(&right.file));
    selected
}
