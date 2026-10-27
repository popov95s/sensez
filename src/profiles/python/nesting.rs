//! Python block nesting, branch and cognitive-complexity weights.

/// Kinds that increase block-nesting depth.
pub(super) fn is_nesting(kind: &str) -> bool {
    matches!(
        kind,
        "if_statement" | "for_statement" | "while_statement" | "try_statement" | "with_statement"
    )
}

/// Kinds counted as cyclomatic decision points.
pub(super) fn is_branch(kind: &str) -> bool {
    matches!(
        kind,
        "if_statement"
            | "elif_clause"
            | "for_statement"
            | "while_statement"
            | "except_clause"
            | "boolean_operator"
            | "conditional_expression"
            | "if_clause"
    )
}

/// Cognitive-complexity increment (Sonar-style): control structures cost
/// `1 + nesting`; boolean operators and `elif` cost a flat `1`.
pub(super) fn cognitive_weight(kind: &str, depth: usize) -> Option<usize> {
    match kind {
        "if_statement" | "for_statement" | "while_statement" | "conditional_expression" => {
            Some(1 + depth)
        }
        "except_clause" => Some(1 + depth),
        "elif_clause" | "boolean_operator" => Some(1),
        _ => None,
    }
}
