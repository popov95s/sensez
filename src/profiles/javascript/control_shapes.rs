//! JS/TS branch and guard-clause shapes for the shared control-flow helpers.

use crate::profiles::branch_path::BranchArms;
use crate::profiles::guard_clause::GuardClauseSyntax;

pub(super) const BRANCH_ARMS: BranchArms = BranchArms {
    fields: &[("if_statement", &["consequence", "alternative"])],
    children: &["switch_body"],
};

pub(super) const GUARD_CLAUSE: GuardClauseSyntax = GuardClauseSyntax {
    if_kind: "if_statement",
    block_kinds: &["statement_block"],
    wrapper_kinds: &[],
    is_function: super::units::is_function,
};
