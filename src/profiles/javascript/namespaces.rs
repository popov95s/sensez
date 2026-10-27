//! Narrow `import * as ns` to the members read through `ns.member`.

use crate::profiles::node_names::NodeNames;
use crate::spine::ir::{ImportContext, Walked};
use std::collections::HashSet;
use tree_sitter::Node;

/// Names read other than as `ns.member` / `ns.Type`, gathered as the walk visits them.
#[derive(Default)]
pub struct FreeReads {
    names: HashSet<String>,
}

impl FreeReads {
    /// Record `node` when it is a free read under `ancestors`, root first.
    pub fn record(&mut self, node: Node, ancestors: &[Node], src: &[u8]) {
        if !matches!(
            node.kind_name(),
            "identifier" | "shorthand_property_identifier"
        ) || ancestors
            .last()
            .is_some_and(|parent| is_member_base(node, *parent))
        {
            return;
        }
        let text = node.utf8_text(src).unwrap_or_default();
        if !self.names.contains(text)
            && !ancestors
                .iter()
                .any(|ancestor| ancestor.kind_name() == "import_statement")
        {
            self.names.insert(text.to_string());
        }
    }

    /// A re-export the walk does not descend into still reads its names.
    pub fn skipped(&mut self, node: Node, src: &[u8]) {
        if node.kind_name() != "export_statement" {
            return;
        }
        let mut stack = vec![(node, None)];
        while let Some((current, parent)) = stack.pop() {
            if current.kind_name() == "import_statement" {
                continue;
            }
            self.record(current, parent.as_slice(), src);
            let mut cursor = current.walk();
            stack.extend(
                current
                    .named_children(&mut cursor)
                    .map(|child| (child, Some(current))),
            );
        }
    }
}

/// Drop the `*` of namespace imports whose binding is only used as `ns.member` / `ns.Type`.
pub fn narrow(free_reads: &FreeReads, imports: &mut [ImportContext]) {
    for import in imports.iter_mut().filter(|import| is_namespace(import)) {
        if !free_reads.names.contains(&import.bindings[0]) {
            import.imported_symbols.clear();
        }
    }
}

fn is_namespace(import: &ImportContext) -> bool {
    !import.is_reexport && import.imported_symbols == ["*"] && import.bindings.len() == 1
}

fn is_member_base(node: Node, parent: Node) -> bool {
    let field = match parent.kind_name() {
        "member_expression" => "object",
        "nested_type_identifier" => "module",
        _ => return false,
    };
    parent
        .field(field)
        .is_some_and(|base| base.id() == node.id())
}

/// A hop through an `import * as ns` binding is a module path, not navigation.
pub fn discount_namespace_hops(out: &mut Walked) {
    let namespaces: HashSet<&str> = out
        .symbols
        .imports
        .iter()
        .flat_map(|import| import.imported_symbols.iter().zip(&import.bindings))
        .filter(|(symbol, _)| *symbol == "*")
        .map(|(_, binding)| binding.as_str())
        .collect();
    if namespaces.is_empty() {
        return;
    }
    for hit in out
        .units
        .functions
        .iter_mut()
        .flat_map(|f| f.chain_hits.iter_mut())
    {
        if hit.root.as_deref().is_some_and(|r| namespaces.contains(r)) {
            hit.depth = hit.depth.saturating_sub(1);
        }
    }
}
