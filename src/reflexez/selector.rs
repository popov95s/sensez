use super::changes;
use super::discovery::{self, TestFile};
use super::dynamic::{self, DynamicFacts};
use super::model::{ImpactPlan, PlanReason, Selection};
use super::selector_dynamic::{self, Reach};
use crate::cli::spec::ReflexezArgs;
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
    let (config, config_issues) = crate::config::model::Config::load_for_scan(&scope.repository);
    let graph = crate::spine::graph::build(&parsed.files, &config.roots);
    let mut fallback = scope.fallback_reasons;
    if !config_issues.is_empty() || !parsed.issues.is_empty() {
        fallback.push("incomplete parsing could hide an affected test".into());
    }
    if args.full {
        fallback.push("full-suite execution was explicitly requested".into());
    }
    let tests_by_path = tests_by_path(&project.tests);
    let nodes_by_path = nodes_by_path(&graph);
    let dynamic_reverse =
        selector_dynamic::reverse_edges(&graph, &dynamic, &nodes_by_path, &scope.repository);
    let mut selected = selected_reach(
        &scope.files,
        &tests_by_path,
        &graph,
        &nodes_by_path,
        &dynamic_reverse,
        &mut fallback,
    );
    let selected_paths: HashSet<_> = selected.keys().cloned().collect();
    let relevant_unresolved = dynamic
        .by_file
        .iter()
        .filter(|(path, _)| scope.files.contains(path) || selected_paths.contains(*path))
        .map(|(_, facts)| facts.unresolved)
        .sum::<usize>();
    if relevant_unresolved > 0 {
        fallback.push(format!(
            "{relevant_unresolved} relevant computed dynamic import(s) could not be resolved safely"
        ));
    }
    if args.strict_dynamic && dynamic.unresolved > 0 {
        fallback.push(format!(
            "strict dynamic safety found {} unresolved computed import(s)",
            dynamic.unresolved
        ));
    }
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
    let selected_paths: HashSet<_> = selected.keys().cloned().collect();
    let selections = selections(selected, &tests_by_path, full_suite, args.full);
    let runners = super::runners::plans(&scope.repository, &project.tests, &selected_paths);
    Ok(ImpactPlan {
        repository: scope.repository,
        changed_files: scope.files,
        discovered_tests: project.tests.len(),
        selected: selections,
        runners,
        full_suite,
        fallback_reasons: fallback,
        unresolved_dynamic_imports: dynamic.unresolved,
        selection_ms: started.elapsed().as_millis(),
    })
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

fn selected_reach(
    changed: &[PathBuf],
    tests: &HashMap<PathBuf, &TestFile>,
    graph: &CodebaseGraph,
    nodes: &HashMap<PathBuf, NodeIndex>,
    dynamic_reverse: &HashMap<NodeIndex, Vec<(NodeIndex, bool)>>,
    fallback: &mut Vec<String>,
) -> HashMap<PathBuf, Reach> {
    let static_reach = crate::spine::impact::reachable_files(
        graph,
        changed,
        crate::spine::impact::DirectionOfImpact::Dependents,
        crate::spine::impact::ImpactOptions {
            include_type_only: false,
        },
    );
    let test_nodes: HashMap<_, _> = tests
        .keys()
        .filter_map(|path| nodes.get(path).map(|index| (*index, path.clone())))
        .collect();
    let mut selected: HashMap<_, _> = static_reach
        .files
        .into_iter()
        .filter(|(path, _)| tests.contains_key(path))
        .map(|(path, distance)| {
            (
                path,
                Reach {
                    distance,
                    dynamic: false,
                },
            )
        })
        .collect();
    for path in static_reach.unmapped {
        if is_source(&path) && !path.exists() {
            fallback.push("a changed source file is unavailable for graph analysis".into());
        }
    }
    let mut source_changes = 0;
    for path in changed {
        if tests.contains_key(path) {
            selected.insert(
                path.clone(),
                Reach {
                    distance: 0,
                    dynamic: false,
                },
            );
        }
        let Some(&start) = nodes.get(path) else {
            if is_source(path) && !path.exists() {
                fallback.push("a changed source file is unavailable for graph analysis".into());
            }
            continue;
        };
        source_changes += 1;
        selector_dynamic::walk_reverse(start, dynamic_reverse, &test_nodes, &mut selected);
    }
    fallback.sort();
    fallback.dedup();
    if source_changes > 0 && selected.is_empty() {
        fallback.push("changed source code has no provable test dependency".into());
    }
    selected
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

fn is_source(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("py" | "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx")
    )
}
