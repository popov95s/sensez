//! Which functions defined inside another function count as nested helpers.

use crate::profiles::lineage::Lineage;
use crate::profiles::node_names::NodeNames;

/// `function f() {}`, or `const f = () => {}` outside a component or hook.
pub(super) fn is_named_local_function(node: Lineage, kind: &str, parent: &str) -> bool {
    match kind {
        "function_declaration" | "generator_function_declaration" => true,
        "arrow_function" | "function_expression" => {
            node.parent(node.node())
                .is_some_and(|binding| binding.kind_name() == "variable_declarator")
                && !is_component_or_hook(parent)
        }
        _ => false,
    }
}

/// `SaveButton` or `useSave`.
fn is_component_or_hook(name: &str) -> bool {
    let hook = name
        .strip_prefix("use")
        .is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_uppercase()));
    hook || name.starts_with(|c: char| c.is_ascii_uppercase())
}
