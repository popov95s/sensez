use super::matcher::{compile, matches_name, node_matches, Matcher};
use crate::config::model::ForbiddenRule;
use crate::report::{ActionLevel, BoundaryViolation};
use crate::spine::graph::CodebaseGraph;
use petgraph::visit::EdgeRef;

/// Result of the boundary audit.
#[derive(Default)]
pub struct BoundaryAudit {
    pub violations: Vec<BoundaryViolation>,
    /// Rule labels whose `from` side matched no module in the scan.
    pub unmatched_rules: Vec<String>,
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

    // The `to` side also tests the literal import target: unresolved imports must still match.
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
