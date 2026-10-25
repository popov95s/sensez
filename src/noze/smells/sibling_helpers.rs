//! Sibling helper redefinition: one small helper written again in another module.
//!
//! Module-level functions are grouped by clone family, name and arity; members
//! in different modules whose lexeme bodies reach `sibling_helper_min_similarity`
//! percent similarity form one finding, anchored at the first definition.

use super::make;
use super::union_find::{find, union};
use crate::config::smells::SmellConfig;
use crate::noze::duplication::{function_lexemes, length_ceiling_percent, similarity_percent};
use crate::profiles::registry;
use crate::report::{Severity, SmellFinding, SmellKind};
use crate::spine::graph::CodebaseGraph;
use crate::spine::ir::{CallbackPosition, Language, SymbolKind};
use crate::spine::parser::{FunctionUnit, ParsedFile};
use petgraph::visit::EdgeRef;
use rustc_hash::FxHashSet;
use std::collections::BTreeMap;
use std::path::Path;

struct Definition<'a> {
    file: &'a ParsedFile,
    func: &'a FunctionUnit,
    lexemes: &'a [u64],
}

type GroupKey<'a> = (Language, &'a str, usize);
type ImportLink<'a> = (&'a Path, &'a Path, &'a str);

pub fn detect(
    files: &[&ParsedFile],
    graph: &CodebaseGraph,
    cfg: &SmellConfig,
) -> Vec<SmellFinding> {
    let groups = definition_groups(files, cfg);
    let links = import_links(graph);
    groups
        .into_values()
        .flat_map(|group| similar_clusters(without_wrappers(group, &links), cfg))
        .map(|cluster| finding(&cluster))
        .collect()
}

fn definition_groups<'a>(
    files: &[&'a ParsedFile],
    cfg: &SmellConfig,
) -> BTreeMap<GroupKey<'a>, Vec<Definition<'a>>> {
    let mut groups: BTreeMap<GroupKey<'a>, Vec<Definition<'a>>> = BTreeMap::new();
    for file in files.iter().copied().filter(|file| is_helper_module(file)) {
        let settings = cfg.for_language(file.language);
        if !settings.is_enabled(SmellKind::SiblingHelperRedefinition) {
            continue;
        }
        for func in file
            .walked
            .units
            .functions
            .iter()
            .filter(|f| is_helper(file, f))
        {
            let lexemes = function_lexemes(file, func);
            if lexemes.len() < settings.sibling_helper_min_lexemes {
                continue;
            }
            let key = (
                registry::clone_family(file.language),
                func.name.as_str(),
                func.param_names.len(),
            );
            groups.entry(key).or_default().push(Definition {
                file,
                func,
                lexemes,
            });
        }
    }
    groups.retain(|_, group| distinct_files(group) >= 2);
    groups
}

fn is_helper_module(file: &ParsedFile) -> bool {
    !registry::is_test_or_support_source(file.language, &file.path)
        && !registry::module_profile(file.language).is_package_index(&file.path)
}

fn is_helper(file: &ParsedFile, func: &FunctionUnit) -> bool {
    let starts_upper = func.name.chars().next().is_some_and(char::is_uppercase);
    is_module_declaration(file, func)
        && !func.is_method
        && !starts_upper
        && !registry::smell_wording(file.language)
            .framework_names
            .contains(&func.name.as_str())
}

/// A named module-level function or function-valued binding declared at this line.
fn is_module_declaration(file: &ParsedFile, func: &FunctionUnit) -> bool {
    let symbols = &file.walked.symbols;
    !func.is_nested
        && func.callback_position == CallbackPosition::Standalone
        && symbols.declared_lines.get(&func.name) == Some(&func.start_line)
        && matches!(
            symbols.declared_kinds.get(&func.name),
            Some(SymbolKind::Function | SymbolKind::Variable)
        )
}

/// `(importer, imported module, name)` for every named import between project files.
fn import_links(graph: &CodebaseGraph) -> FxHashSet<ImportLink<'_>> {
    graph
        .graph
        .edge_references()
        .flat_map(|edge| {
            let from = graph.graph[edge.source()].file_path.as_path();
            let to = graph.graph[edge.target()].file_path.as_path();
            edge.weight()
                .imported_symbols
                .iter()
                .map(move |name| (from, to, name.as_str()))
        })
        .collect()
}

/// Drop definitions whose module imports the name (or the whole module) from a sibling.
fn without_wrappers<'a>(
    group: Vec<Definition<'a>>,
    links: &FxHashSet<ImportLink<'_>>,
) -> Vec<Definition<'a>> {
    let paths: Vec<&Path> = group.iter().map(|d| d.file.path.as_path()).collect();
    group
        .into_iter()
        .filter(|definition| {
            let own = definition.file.path.as_path();
            !paths.iter().filter(|&&other| other != own).any(|&other| {
                links.contains(&(own, other, definition.func.name.as_str()))
                    || links.contains(&(own, other, "*"))
            })
        })
        .collect()
}

/// Components of definitions in different files whose bodies are similar enough.
fn similar_clusters<'a>(group: Vec<Definition<'a>>, cfg: &SmellConfig) -> Vec<Vec<Definition<'a>>> {
    let Some(first) = group.first() else {
        return Vec::new();
    };
    let threshold = cfg
        .for_language(first.file.language)
        .sibling_helper_min_similarity;
    let mut parent: Vec<usize> = (0..group.len()).collect();
    for left in 0..group.len() {
        for right in left + 1..group.len() {
            if similar(&group[left], &group[right], threshold) {
                union(&mut parent, left, right);
            }
        }
    }
    let mut clusters: BTreeMap<usize, Vec<Definition<'a>>> = BTreeMap::new();
    for (index, definition) in group.into_iter().enumerate() {
        clusters
            .entry(find(&mut parent, index))
            .or_default()
            .push(definition);
    }
    clusters
        .into_values()
        .filter(|cluster| distinct_files(cluster) >= 2)
        .collect()
}

fn similar(left: &Definition, right: &Definition, threshold: usize) -> bool {
    left.file.path != right.file.path
        && !is_forked_module(&left.file.path, &right.file.path)
        && length_ceiling_percent(left.lexemes, right.lexemes) >= threshold
        && similarity_percent(left.lexemes, right.lexemes) >= threshold
}

/// Same file name in a same-named directory: a whole-module fork the clone pillar reports.
fn is_forked_module(left: &Path, right: &Path) -> bool {
    left.file_name() == right.file_name() && parent_name(left) == parent_name(right)
}

fn parent_name(path: &Path) -> Option<&std::ffi::OsStr> {
    path.parent().and_then(Path::file_name)
}

fn distinct_files(group: &[Definition]) -> usize {
    group
        .iter()
        .map(|definition| definition.file.path.as_path())
        .collect::<FxHashSet<_>>()
        .len()
}

fn finding(cluster: &[Definition]) -> SmellFinding {
    let mut members: Vec<&Definition> = cluster.iter().collect();
    members.sort_by_key(|d| (d.file.path.as_path(), d.func.start_line));
    let anchor = members[0];
    let root = common_root(&members);
    let others: Vec<String> = members[1..]
        .iter()
        .map(|d| format!("{}:L{}", relative(&d.file.path, root), d.func.start_line))
        .collect();
    let arity = anchor.func.param_names.len();
    make(
        SmellKind::SiblingHelperRedefinition,
        format!(
            "`{}` ({arity} params) is defined again with a near-identical body in {} — keep one and import it",
            anchor.func.name,
            others.join(", "),
        ),
        &anchor.file.path,
        anchor.func.start_line,
        &anchor.func.name,
        Severity::Info,
        members.len() as u32,
        2,
    )
}

fn common_root<'a>(members: &[&'a Definition]) -> &'a Path {
    let first = members[0].file.path.as_path();
    first
        .ancestors()
        .find(|ancestor| members.iter().all(|d| d.file.path.starts_with(ancestor)))
        .unwrap_or(first)
}

fn relative(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}
