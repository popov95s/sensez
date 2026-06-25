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
use globset::{Glob, GlobSet};
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
        let mut builder = GlobSet::builder();
        if let Ok(glob) = Glob::new(pattern) {
            builder.add(glob);
        }
        Matcher::Glob(builder.build().unwrap_or_else(|_| GlobSet::empty()))
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

/// Match a dotted name: prefix (equal or submodule) or glob. For globs the
/// name is tried both as-is (so `*.db.*` matches `app.db.session`) and with
/// dots rewritten to slashes (so a *path* glob like `**/db/**` matches the same
/// dotted module/import target even when it didn't resolve to a file on disk).
fn matches_name(matcher: &Matcher, name: &str) -> bool {
    match matcher {
        Matcher::Prefix(prefix) => name
            .strip_prefix(prefix.as_str())
            .is_some_and(|rest| rest.is_empty() || rest.starts_with('.')),
        Matcher::Glob(set) => {
            set.is_match(Path::new(name)) || set.is_match(Path::new(&name.replace('.', "/")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spine::parser::parse_file;
    use std::fs;

    #[test]
    fn flags_forbidden_edge() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_path_buf();
        fs::create_dir_all(dir.join("domain")).unwrap();
        fs::create_dir_all(dir.join("web")).unwrap();
        fs::write(dir.join("domain/__init__.py"), "").unwrap();
        fs::write(dir.join("web/__init__.py"), "").unwrap();
        fs::write(dir.join("domain/core.py"), "from web.views import render\n").unwrap();
        fs::write(dir.join("web/views.py"), "def render():\n    pass\n").unwrap();

        let names = [
            "domain/__init__.py",
            "web/__init__.py",
            "domain/core.py",
            "web/views.py",
        ];
        let files: Vec<_> = names
            .iter()
            .enumerate()
            .map(|(i, n)| parse_file(&dir.join(n), i as u32).unwrap())
            .collect();
        let cg = crate::spine::graph::build(&files, &[]);

        // Plain prefix rule (original behavior).
        let prefix = audit(&cg, &[rule("domain", "web")]);
        assert_eq!(prefix.violations.len(), 1);
        assert_eq!(prefix.violations[0].from_module, "domain.core");
        assert!(prefix.unmatched_rules.is_empty());

        // Glob on module name.
        let glob = audit(&cg, &[rule("domain.*", "web.*")]);
        assert_eq!(glob.violations.len(), 1, "glob on module name matches");

        // Glob on file path — robust to module naming.
        let path = audit(&cg, &[rule("**/domain/**", "**/web/**")]);
        assert_eq!(path.violations.len(), 1, "glob on file path matches");

        // A rule that matches no module is reported, not silently "clean".
        let dead = audit(&cg, &[rule("nonexistent.layer", "web")]);
        assert!(dead.violations.is_empty());
        assert_eq!(dead.unmatched_rules.len(), 1, "dead rule surfaced");
    }

    fn rule(from: &str, to: &str) -> ForbiddenRule {
        ForbiddenRule {
            from: from.into(),
            to: to.into(),
        }
    }

    /// Regression: a namespace-package layout (no `__init__.py`, no `roots`)
    /// where the import target doesn't resolve to an in-repo node is still
    /// caught by a path-glob `from` + a dotted-glob `to` (matched against the
    /// literal import string).
    #[test]
    fn matches_namespace_layout_via_path_and_literal_target() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_path_buf();
        fs::create_dir_all(dir.join("app/api/endpoints")).unwrap();
        fs::create_dir_all(dir.join("app/db")).unwrap();
        fs::write(
            dir.join("app/api/endpoints/users.py"),
            "from app.db.models import Row\n",
        )
        .unwrap();
        fs::write(dir.join("app/db/models.py"), "class Row:\n    pass\n").unwrap();

        let files: Vec<_> = ["app/api/endpoints/users.py", "app/db/models.py"]
            .iter()
            .enumerate()
            .map(|(i, n)| parse_file(&dir.join(n), i as u32).unwrap())
            .collect();
        let cg = crate::spine::graph::build(&files, &[]);

        // dotted `to` glob matches the literal import string.
        let dotted = audit(&cg, &[rule("**/api/endpoints/**", "app.db.*")]);
        assert_eq!(dotted.violations.len(), 1, "path-from + dotted-to matches");
        assert!(dotted.unmatched_rules.is_empty());

        // path `to` glob matches the dotted import via dot→slash — the rule
        // shape that previously failed on a namespace (no __init__.py) layout.
        let path = audit(&cg, &[rule("**/endpoints/**", "**/db/**")]);
        assert_eq!(
            path.violations.len(),
            1,
            "path-from + path-to matches an unresolved import"
        );
    }
}
