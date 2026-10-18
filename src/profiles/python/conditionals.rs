use crate::profiles::conditionals::{self, IfShape, TernaryShape};
use crate::spine::ir::NestedTernary;
use tree_sitter::Node;

const IF_SHAPE: IfShape<'static> = IfShape {
    if_kind: "if_statement",
    then_field: "consequence",
    else_field: "alternative",
    block_kinds: &["block"],
    ignored_kinds: &["comment"],
};

pub(crate) fn is_collapsible_nested_if(node: Node<'_>) -> bool {
    conditionals::is_collapsible_nested_if(node, &IF_SHAPE)
}

const TERNARY_SHAPE: TernaryShape<'static> = TernaryShape {
    conditional_kind: "conditional_expression",
    transparent_kinds: &[
        "parenthesized_expression",
        "not_operator",
        "boolean_operator",
        "binary_operator",
        "unary_operator",
        "comparison_operator",
        "await",
    ],
    function_kinds: &["function_definition", "lambda"],
};

pub(crate) fn nested_ternary(node: Node<'_>) -> Option<NestedTernary> {
    conditionals::ternary_nesting(node, &TERNARY_SHAPE)
}
