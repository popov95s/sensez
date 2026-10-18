//! Narrow `import * as ns` to the members read through `ns.member`.

use crate::spine::ir::ImportContext;
use tree_sitter::Node;

/// Drop the `*` of namespace imports whose binding is only used as `ns.member` / `ns.Type`.
pub fn narrow(root: Node, src: &[u8], imports: &mut [ImportContext]) {
    for import in imports.iter_mut().filter(|import| is_namespace(import)) {
        if !escapes(root, src, &import.bindings[0]) {
            import.imported_symbols.clear();
        }
    }
}

fn is_namespace(import: &ImportContext) -> bool {
    !import.is_reexport && import.imported_symbols == ["*"] && import.bindings.len() == 1
}

fn escapes(root: Node, src: &[u8], binding: &str) -> bool {
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if node.kind() == "import_statement" {
            continue;
        }
        if matches!(node.kind(), "identifier" | "shorthand_property_identifier")
            && node.utf8_text(src).is_ok_and(|text| text == binding)
            && !is_member_base(node)
        {
            return true;
        }
        let mut cursor = node.walk();
        stack.extend(node.named_children(&mut cursor));
    }
    false
}

fn is_member_base(node: Node) -> bool {
    node.parent().is_some_and(|parent| {
        let field = match parent.kind() {
            "member_expression" => "object",
            "nested_type_identifier" => "module",
            _ => return false,
        };
        parent
            .child_by_field_name(field)
            .is_some_and(|base| base.id() == node.id())
    })
}
