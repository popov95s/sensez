use crate::profiles::conditionals::{self, IfShape};
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

pub(crate) fn is_nested_ternary(node: Node<'_>) -> bool {
    conditionals::is_nested_conditional(
        node,
        "ternary_expression",
        &[
            "function_declaration",
            "function_expression",
            "function",
            "arrow_function",
            "generator_function",
            "generator_function_declaration",
            "method_definition",
        ],
    )
}
