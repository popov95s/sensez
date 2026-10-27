//! Shared matching of profile-owned test-check syntax.

use crate::profiles::node_names::NodeNames;
use tree_sitter::Node;

pub(crate) enum CallPattern {
    Exact(&'static str),
    Prefix(&'static str),
    MemberExact {
        callee_prefix: &'static str,
        member: &'static str,
    },
    MemberPrefix {
        callee_prefix: &'static str,
        member_prefix: &'static str,
    },
}

pub(crate) struct TestChecks {
    pub statement_kinds: &'static [&'static str],
    pub call_kind: &'static str,
    pub callee_field: &'static str,
    pub member_kind: Option<&'static str>,
    pub member_field: Option<&'static str>,
    pub calls: &'static [CallPattern],
    /// Framework check roots that never count as helper calls on their own (`expect`).
    pub framework_calls: &'static [&'static str],
}

/// Callee name of a plain call to a helper function, excluding framework check roots.
pub(crate) fn helper_call<'a>(
    node: Node<'_>,
    source: &'a [u8],
    checks: &TestChecks,
) -> Option<&'a str> {
    if node.kind_name() != checks.call_kind {
        return None;
    }
    node.field(checks.callee_field)
        .filter(|callee| callee.kind_name() == "identifier")
        .and_then(|callee| callee.utf8_text(source).ok())
        .filter(|name| !checks.framework_calls.contains(name))
}

pub(crate) fn is_check(node: Node<'_>, source: &[u8], checks: &TestChecks) -> bool {
    if checks.statement_kinds.contains(&node.kind_name()) {
        return true;
    }
    if node.kind_name() != checks.call_kind {
        return false;
    }
    let Some(callee) = node.field(checks.callee_field) else {
        return false;
    };
    let Ok(name) = callee.utf8_text(source) else {
        return false;
    };
    let member = checks
        .member_kind
        .filter(|kind| *kind == callee.kind_name())
        .and(checks.member_field)
        .and_then(|field| callee.field(field))
        .and_then(|property| property.utf8_text(source).ok());
    checks.calls.iter().any(|pattern| match pattern {
        CallPattern::Exact(exact) => name == *exact,
        CallPattern::Prefix(prefix) => name.starts_with(prefix),
        CallPattern::MemberExact {
            callee_prefix,
            member: expected,
        } => name.starts_with(callee_prefix) && member == Some(*expected),
        CallPattern::MemberPrefix {
            callee_prefix,
            member_prefix,
        } => {
            name.starts_with(callee_prefix)
                && member.is_some_and(|value| value.starts_with(member_prefix))
        }
    })
}
