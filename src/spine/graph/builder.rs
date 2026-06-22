//! Assemble a [`CodebaseGraph`] from parsed files.

use crate::profiles::{registry, Language};
use crate::spine::graph::{CodebaseGraph, ModuleNode};
use crate::spine::parser::ParsedFile;
use petgraph::graph::NodeIndex;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Build the directed module graph. `configured_roots` overrides auto-detection.
pub fn build(files: &[ParsedFile], configured_roots: &[PathBuf]) -> CodebaseGraph {
    let mut cg = CodebaseGraph::default();
    // Per file: (resolved module name, detected root). Roots are memoized per
    // (parent dir, language) so a per-file marker walk becomes a per-dir one.
    let mut module_of: Vec<Option<String>> = Vec::with_capacity(files.len());
    let mut root_of: Vec<PathBuf> = Vec::with_capacity(files.len());
    let mut root_cache: HashMap<(PathBuf, Language), PathBuf> = HashMap::new();

    // Pass 1: create a node per file. Re-exports are NOT folded into declared
    // symbols — a re-exported name is kept alive in its defining module by the
    // re-export *edge*, and folding it here would falsely flag the package's
    // re-export as dead.
    for file in files {
        let root = root_for(file, configured_roots, &mut root_cache);
        let name = registry::module_profile(file.language).module_name(&file.path, &root);
        if cg.name_to_index.contains_key(&name) {
            // Same logical module identity, e.g. app.py and app/__init__.py.
            // Keep the first node so imports remain deterministic.
            module_of.push(None);
            root_of.push(root);
            continue;
        }
        let idx = cg.graph.add_node(ModuleNode {
            file_path: file.path.clone(),
            module_name: name.clone(),
            language: file.language,
            declared_public_symbols: file.walked.symbols.declared.clone(),
            declared_kinds: file.walked.symbols.declared_kinds.clone(),
            declared_lines: file.walked.symbols.declared_lines.clone(),
            dunder_all: file.walked.symbols.dunder_all.clone(),
            decorators: file.walked.symbols.decorators.clone(),
            name_counts: file.walked.usage.name_counts.clone(),
            is_external: false,
        });
        cg.name_to_index.insert(name.clone(), idx);
        module_of.push(Some(name));
        root_of.push(root);
    }

    // Pass 2: add an edge per import.
    for (i, file) in files.iter().enumerate() {
        let Some(module_name) = module_of[i].as_ref() else {
            continue;
        };
        let profile = registry::module_profile(file.language);
        let src_idx = cg.name_to_index[module_name];
        let is_index = profile.is_package_index(&file.path);
        let pkg = profile.containing_package(module_name, is_index);
        for import in &file.walked.symbols.imports {
            let target = profile.resolve_target(import, &pkg, &file.path, &root_of[i]);
            add_import_edges(
                &mut cg,
                src_idx,
                file.language,
                &target,
                import,
                &file.walked.usage.attribute_accesses,
            );
        }
    }
    cg
}

/// Resolve a file's package root: the longest configured root that contains it,
/// else the profile's auto-detected root (memoized per parent dir + language).
fn root_for(
    file: &ParsedFile,
    configured: &[PathBuf],
    cache: &mut HashMap<(PathBuf, Language), PathBuf>,
) -> PathBuf {
    if let Some(root) = configured
        .iter()
        .filter(|r| file.path.starts_with(r))
        .max_by_key(|r| r.components().count())
    {
        return root.clone();
    }
    let dir = file.path.parent().unwrap_or(Path::new(".")).to_path_buf();
    cache
        .entry((dir, file.language))
        .or_insert_with(|| registry::module_profile(file.language).root_for(&file.path))
        .clone()
}

/// Add edge(s) for one import. `from pkg import name` where `pkg.name` is a
/// *submodule* resolves to that submodule, not to a `name` symbol on the
/// package. Crucially, the symbols accessed on the bound name via attribute
/// access (`crud.fetch(...)`) are credited to that edge — so a module used only
/// through `module.func()` isn't falsely flagged dead.
fn add_import_edges(
    cg: &mut CodebaseGraph,
    src_idx: NodeIndex,
    src_lang: Language,
    target: &str,
    import: &crate::spine::parser::ImportContext,
    attrs: &HashMap<String, HashSet<String>>,
) {
    let profile = registry::module_profile(src_lang);
    let src_module = cg.graph[src_idx].module_name.clone();
    let add_edge =
        |cg: &mut CodebaseGraph, dst: NodeIndex, mut ctx: crate::spine::parser::ImportContext| {
            ctx.is_module_decl =
                ctx.is_module_decl || profile.is_containment(&src_module, &ctx.target_module);
            cg.graph.add_edge(src_idx, dst, ctx);
        };
    let mut package_symbols: Vec<String> = Vec::new();
    for (i, symbol) in import.imported_symbols.iter().enumerate() {
        if symbol == "*" {
            package_symbols.push(symbol.clone());
            continue;
        }
        let submodule = profile.submodule_candidate(target, symbol);
        if let Some(submodule) = submodule.filter(|s| cg.name_to_index.contains_key(s)) {
            let binding = import
                .bindings
                .get(i)
                .map_or(symbol.as_str(), String::as_str);
            let dst = node_for_target(cg, src_lang, &submodule);
            let ctx = qualified_import(import, &submodule, attrs.get(binding));
            add_edge(cg, dst, ctx);
        } else {
            package_symbols.push(symbol.clone());
        }
    }
    // Plain `import x` / `import x as y`: credit attrs accessed via the bound name.
    if import.imported_symbols.is_empty() {
        let used = import.bindings.first().and_then(|b| attrs.get(b));
        let dst = node_for_target(cg, src_lang, target);
        let ctx = qualified_import(import, target, used);
        add_edge(cg, dst, ctx);
    } else if !package_symbols.is_empty() {
        let dst = node_for_target(cg, src_lang, target);
        let mut ctx = import.clone();
        ctx.target_module = target.to_string();
        ctx.imported_symbols = package_symbols;
        add_edge(cg, dst, ctx);
    }
}

/// An edge to `module` whose used symbols are the attributes accessed on the
/// bound name (empty if none) — the precise attribute-access credit.
fn qualified_import(
    base: &crate::spine::parser::ImportContext,
    module: &str,
    accessed: Option<&HashSet<String>>,
) -> crate::spine::parser::ImportContext {
    let mut ctx = base.clone();
    ctx.target_module = module.to_string();
    ctx.imported_symbols = accessed
        .map(|s| s.iter().cloned().collect())
        .unwrap_or_default();
    ctx
}

/// Look up a target node, creating a synthetic external node if unresolved.
/// Synthetic nodes inherit the importing module's language (irrelevant to the
/// analyzers, which skip external nodes).
fn node_for_target(cg: &mut CodebaseGraph, src_lang: Language, target: &str) -> NodeIndex {
    // Deliberately get-then-insert rather than `entry()`: hits (the common
    // case — most imports point at already-known modules) stay allocation-free,
    // whereas `entry(target.to_string())` would allocate the key on every call.
    if let Some(&idx) = cg.name_to_index.get(target) {
        return idx;
    }
    let idx = cg.graph.add_node(ModuleNode {
        file_path: PathBuf::new(),
        module_name: target.to_string(),
        language: src_lang,
        declared_public_symbols: Vec::new(),
        declared_kinds: HashMap::new(),
        declared_lines: HashMap::new(),
        dunder_all: None,
        decorators: HashMap::new(),
        name_counts: HashMap::new(),
        is_external: true,
    });
    cg.name_to_index.insert(target.to_string(), idx);
    idx
}
