//! Config-driven import-boundary auditing.
//!
//! Walks every edge of the prebuilt graph and reports any that matches a
//! forbidden rule from `sensez.toml`. A rule's `from`/`to` is matched against
//! both the **module name** and the **file path** of each endpoint, so layers
//! can be expressed either way (`app.api.*` or `**/api/**`) and path patterns
//! remain robust to namespace-package naming. A plain pattern (no glob
//! metacharacters) keeps the original dotted-prefix behavior. Pure in-memory.

use crate::config::model::ForbiddenRule;
use crate::report::{ActionLevel, BoundaryViolation};
use crate::spine::graph::{CodebaseGraph, ModuleNode};
use globset::GlobSet;
use petgraph::visit::EdgeRef;
use std::path::Path;

/// Result of the boundary audit.
#[derive(Default)]
pub struct BoundaryAudit {
    pub violations: Vec<BoundaryViolation>,
    /// Rule labels whose `from` side matched no module in the scan — almost
    /// always a typo or stale pattern, surfaced so it isn't mistaken for clean.
    pub unmatched_rules: Vec<String>,
}

/// A compiled endpoint pattern: dotted-prefix (plain) or glob (name + path).
enum Matcher {
    Prefix(String),
    Glob(GlobSet),
}

/// Report violating edges, plus any rule whose `from` matched nothing.
pub fn audit(cg: &CodebaseGraph, rules: &[ForbiddenRule]) -> BoundaryAudit {
    if rules.is_empty() {
        return BoundaryAudit::default();
    }
    let compiled: Vec<(Matcher, Matcher, String)> = rules
        .iter()
        .map(|r| {
            (
                compile(&r.from),
                compile(&r.to),
                format!("{} -x-> {}", r.from, r.to),
            )
        })
        .collect();

    // Pass 1: which rules' `from` matches at least one in-repo module.
    let mut from_hit = vec![false; compiled.len()];
    for idx in cg.graph.node_indices() {
        let node = &cg.graph[idx];
        if node.is_external {
            continue;
        }
        for (i, (from, ..)) in compiled.iter().enumerate() {
            from_hit[i] |= node_matches(from, node);
        }
    }

    // Pass 2: edges that cross a forbidden boundary. The `to` side also tests
    // the import's *literal* target string, so a rule still matches when the
    // import couldn't be resolved to an in-repo node (namespace-package case).
    let mut violations = Vec::new();
    for edge in cg.graph.edge_references() {
        let src = &cg.graph[edge.source()];
        let dst = &cg.graph[edge.target()];
        let target = &edge.weight().target_module;
        for (from, to, label) in &compiled {
            if node_matches(from, src) && (node_matches(to, dst) || matches_name(to, target)) {
                violations.push(BoundaryViolation {
                    action: ActionLevel::MustFix,
                    from_module: src.module_name.clone(),
                    to_module: dst.module_name.clone(),
                    file: src.file_path.clone(),
                    line: edge.weight().line,
                    rule: label.clone(),
                });
            }
        }
    }

    let unmatched_rules = compiled
        .iter()
        .zip(from_hit)
        .filter(|(_, hit)| !hit)
        .map(|((.., label), _)| label.clone())
        .collect();
    BoundaryAudit {
        violations,
        unmatched_rules,
    }
}

fn compile(pattern: &str) -> Matcher {
    if pattern.contains(['*', '?', '[']) {
        Matcher::Glob(crate::globs::compile_validated(&[pattern]))
    } else {
        Matcher::Prefix(pattern.to_string())
    }
}

/// A node matches on its module name (prefix or glob), or — for globs — on its
/// file path.
fn node_matches(matcher: &Matcher, node: &ModuleNode) -> bool {
    if matches_name(matcher, &node.module_name) {
        return true;
    }
    matches!(matcher, Matcher::Glob(set) if set.is_match(&node.file_path))
}

/// Match a module name: prefix (equal, or a `.`/`/` submodule) or glob. For globs the
/// name is tried both as-is (so `*.db.*` matches `app.db.session`) and with
/// dots rewritten to slashes (so a *path* glob like `**/db/**` matches the same
/// dotted module/import target even when it didn't resolve to a file on disk).
fn matches_name(matcher: &Matcher, name: &str) -> bool {
    match matcher {
        Matcher::Prefix(prefix) => name
            .strip_prefix(prefix.as_str())
            .is_some_and(|rest| rest.is_empty() || rest.starts_with(['.', '/'])),
        Matcher::Glob(set) => {
            set.is_match(Path::new(name)) || set.is_match(Path::new(&name.replace('.', "/")))
        }
    }
}

#[cfg(test)]
mod tests;
