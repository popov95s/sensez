//! Rust branch and guard-clause shapes for the shared control-flow helpers.

use crate::profiles::branch_path::BranchArms;
use crate::profiles::guard_clause::GuardClauseSyntax;

pub(super) const BRANCH_ARMS: BranchArms = BranchArms {
    fields: &[("if_expression", &["consequence", "alternative"])],
    children: &["match_block"],
};

pub(super) const GUARD_CLAUSE: GuardClauseSyntax = GuardClauseSyntax {
    if_kind: "if_expression",
    block_kinds: &["block"],
    wrapper_kinds: &["expression_statement"],
    is_function,
};

pub(super) fn is_function(kind: &str) -> bool {
    matches!(kind, "function_item" | "closure_expression")
}
