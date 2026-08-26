//! Assemble a [`CodebaseGraph`] from parsed files.

use super::identity;
use crate::profiles::{registry, ResolutionCache};
use crate::spine::graph::{CodebaseGraph, ModuleNode};
use crate::spine::ir::Language;
use crate::spine::parser::ParsedFile;
use petgraph::graph::NodeIndex;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Build the directed module graph. `configured_roots` overrides auto-detection.
pub fn build(files: &[ParsedFile], configured_roots: &[PathBuf]) -> CodebaseGraph {
    let mut cg = CodebaseGraph::default();
    let identities = identity::for_files(files, configured_roots);
    let mut module_of: Vec<Option<String>> = Vec::with_capacity(files.len());
    let mut by_scope: HashMap<(Language, PathBuf), HashMap<String, String>> = HashMap::new();
    let mut resolution_cache = ResolutionCache::default();

    // Pass 1: create a node per file. Re-exports are NOT folded into declared
    // symbols — a re-exported name is kept alive in its defining module by the
    // re-export *edge*, and folding it here would falsely flag the package's
    // re-export as dead.
    for (source_index, (file, identity)) in files.iter().zip(&identities).enumerate() {
        if cg.name_to_index.contains_key(&identity.name) {
            // Same logical module identity, e.g. app.py and app/__init__.py.
            // Keep the first node so imports remain deterministic.
            module_of.push(None);
            continue;
        }
        let idx = cg.graph.add_node(ModuleNode {
            file_path: file.path.clone(),
            module_name: identity.name.clone(),
            language: file.language,
            source_index: Some(source_index),
            is_external: false,
        });
        cg.name_to_index.insert(identity.name.clone(), idx);
        by_scope
            .entry((file.language, identity.root.clone()))
            .or_default()
            .insert(identity.logical_name.clone(), identity.name.clone());
        module_of.push(Some(identity.name.clone()));
    }

    // Pass 2: add an edge per import.
    let empty_scope = HashMap::new();
    for (i, file) in files.iter().enumerate() {
        let Some(module_name) = module_of[i].as_ref() else {
            continue;
        };
        let profile = registry::module_profile(file.language);
        let src_idx = cg.name_to_index[module_name];
        let src_module = cg.graph[src_idx].module_name.clone();
        let is_index = profile.is_package_index(&file.path);
        let pkg = profile.containing_package(module_name, is_index);
        let scoped_names = by_scope
            .get(&(file.language, identities[i].root.clone()))
            .unwrap_or(&empty_scope);
        for import in &file.walked.symbols.imports {
            let resolved = profile.resolve_target(
                import,
                &pkg,
                &file.path,
                &identities[i].root,
                &mut resolution_cache,
            );
            // Zero-allocation probe: borrow the canonical name when the
            // logical module exists, else fall back to the raw resolution.
            let target: &str = scoped_names
                .get(resolved.as_str())
                .map(String::as_str)
                .unwrap_or(&resolved);
            add_import_edges(
                &mut cg,
                src_idx,
                &src_module,
                file.language,
                target,
                import,
                &file.walked.usage.attribute_accesses,
            );
        }
    }
    cg
}

/// Add edge(s) for one import. `from pkg import name` where `pkg.name` is a
/// *submodule* resolves to that submodule, not to a `name` symbol on the
/// package. Crucially, the symbols accessed on the bound name via attribute
/// access (`crud.fetch(...)`) are credited to that edge — so a module used only
/// through `module.func()` isn't falsely flagged dead.
fn add_import_edges(
    cg: &mut CodebaseGraph,
    src_idx: NodeIndex,
    src_module: &str,
    src_lang: Language,
    target: &str,
    import: &crate::spine::parser::ImportContext,
    attrs: &HashMap<String, HashSet<String>>,
) {
    let profile = registry::module_profile(src_lang);
    let add_edge =
        |cg: &mut CodebaseGraph, dst: NodeIndex, mut ctx: crate::spine::parser::ImportContext| {
            ctx.is_module_decl =
                ctx.is_module_decl || profile.is_containment(src_module, &ctx.target_module);
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
            add_qualified_edge(
                cg,
                &add_edge,
                src_lang,
                import,
                &submodule,
                attrs.get(binding),
            );
        } else {
            package_symbols.push(symbol.clone());
        }
    }
    // Plain `import x` / `import x as y`: credit attrs accessed via the bound name.
    if import.imported_symbols.is_empty() {
        let used = import.bindings.first().and_then(|b| attrs.get(b));
        add_qualified_edge(cg, &add_edge, src_lang, import, target, used);
    } else if !package_symbols.is_empty() {
        add_package_edge(cg, &add_edge, src_lang, import, target, package_symbols);
    }
}

fn add_qualified_edge(
    cg: &mut CodebaseGraph,
    add_edge: &impl Fn(&mut CodebaseGraph, NodeIndex, crate::spine::parser::ImportContext),
    src_lang: Language,
    import: &crate::spine::parser::ImportContext,
    target: &str,
    used: Option<&HashSet<String>>,
) {
    let dst = node_for_target(cg, src_lang, target);
    let ctx = qualified_import(import, target, used);
    add_edge(cg, dst, ctx);
}

fn add_package_edge(
    cg: &mut CodebaseGraph,
    add_edge: &impl Fn(&mut CodebaseGraph, NodeIndex, crate::spine::parser::ImportContext),
    src_lang: Language,
    import: &crate::spine::parser::ImportContext,
    target: &str,
    package_symbols: Vec<String>,
) {
    let dst = node_for_target(cg, src_lang, target);
    let mut ctx = import.clone();
    ctx.target_module = target.to_string();
    ctx.imported_symbols = package_symbols;
    add_edge(cg, dst, ctx);
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
        source_index: None,
        is_external: true,
    });
    cg.name_to_index.insert(target.to_string(), idx);
    idx
}
