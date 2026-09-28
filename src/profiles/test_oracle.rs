//! Shared matching of profile-owned test-check syntax.

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
}

pub(crate) fn is_check(node: Node<'_>, source: &[u8], checks: &TestChecks) -> bool {
    if checks.statement_kinds.contains(&node.kind()) {
        return true;
    }
    if node.kind() != checks.call_kind {
        return false;
    }
    let Some(callee) = node.child_by_field_name(checks.callee_field) else {
        return false;
    };
    let Ok(name) = callee.utf8_text(source) else {
        return false;
    };
    let member = checks
        .member_kind
        .filter(|kind| *kind == callee.kind())
        .and_then(|_| checks.member_field)
        .and_then(|field| callee.child_by_field_name(field))
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
