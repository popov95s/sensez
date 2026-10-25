use crate::profiles::conditionals::{self, IfShape, TernaryShape};
use crate::spine::ir::NestedTernary;
use tree_sitter::Node;

const IF_SHAPE: IfShape<'static> = IfShape {
    if_kind: "if_statement",
    then_field: "consequence",
    else_field: "alternative",
    block_kinds: &["statement_block"],
    ignored_kinds: &["comment"],
};

pub(crate) fn is_collapsible_nested_if(node: Node<'_>) -> bool {
    conditionals::is_collapsible_nested_if(node, &IF_SHAPE)
}

const TERNARY_SHAPE: TernaryShape<'static> = TernaryShape {
    conditional_kind: "ternary_expression",
    transparent_kinds: &[
        "parenthesized_expression",
        "as_expression",
        "satisfies_expression",
        "non_null_expression",
        "unary_expression",
        "binary_expression",
        "await_expression",
    ],
    function_kinds: &[
        "function_declaration",
        "function_expression",
        "function",
        "arrow_function",
        "generator_function",
        "generator_function_declaration",
        "method_definition",
    ],
    boundary_kinds: &[
        "jsx_element",
        "jsx_self_closing_element",
        "jsx_opening_element",
        "object",
    ],
};

pub(crate) fn nested_ternary(node: Node<'_>) -> Option<NestedTernary> {
    conditionals::ternary_nesting(node, &TERNARY_SHAPE)
}
