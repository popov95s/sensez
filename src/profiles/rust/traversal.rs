//! Pre-order Rust traversal: emits the genericized token stream + spans and
//! extracts imports/declarations while tracking a lexical scope stack.

use super::{imports, lexeme, scope, symbols, tokens as token_map};
use crate::profiles::walk::{self, credit_attr, credit_name, declare, emit_mapped, register_method, Scope};
use crate::spine::ir::{record_attr, Walked};
use std::collections::HashSet;
use tree_sitter::Node;

/// Walk `root` pre-order, producing tokens/spans/imports for the file.
pub fn walk(root: Node, src: &[u8], file_id: u32, module_name: &str) -> Walked {
    walk::run(root, src, file_id, module_name, visit)
}

fn visit(
    node: Node,
    src: &[u8],
    file_id: u32,
    module_name: &str,
    scope: &mut Vec<Scope>,
    fn_bounds: &mut Vec<HashSet<String>>,
    out: &mut Walked,
) {
    let kind = node.kind();

    // `use` declarations (incl. `pub use` re-exports): extracted, not descended
    // into, so import internals never pollute the token stream.
    if kind == "use_declaration" {
        let enclosing = scope.last().map(|s| s.name.as_str());
        out.symbols
            .imports
            .extend(imports::extract(node, src, module_name, enclosing));
        return;
    }

    // `mod name;` (no body): an edge to the child module. Inline `mod x { … }`
    // falls through — it opens a scope and its body is traversed.
    if kind == "mod_item" && node.child_by_field_name("body").is_none() {
        let enclosing = scope.last().map(|s| s.name.as_str());
        if let Some(ctx) = imports::mod_decl(node, src, module_name, enclosing) {
            out.symbols.imports.push(ctx);
        }
        return;
    }

    // Attributes (`#[derive(…)]`, `#![…]`) are metadata, not logic.
    if kind == "attribute_item" || kind == "inner_attribute_item" {
        return;
    }

    // Comments never map to a structural token, so capturing their text for the
    // eyez index cannot affect duplication.
    #[cfg(feature = "eyez")]
    if kind == "line_comment" || kind == "block_comment" {
        let scope_path: Vec<&str> = scope.iter().map(|s| s.name.as_str()).collect();
        crate::eyez::capture::rust::push_comment(out, module_name, &scope_path, node, src);
        return;
    }

    emit_mapped(
        out,
        file_id,
        node,
        src,
        fn_bounds,
        token_map::map_kind,
        lexeme::code,
    );

    if kind == "identifier" || kind == "type_identifier" {
        credit_name(out, node, src);
    }

    // `base.field` / `base.method()` per base identifier, and `base::item`
    // path access — both feed the graph's attribute-access crediting (a module
    // used only as `module::func()` isn't falsely flagged dead).
    if kind == "field_expression" {
        credit_attr(out, node, src, "value", "field");
    }
    if kind == "scoped_identifier" {
        credit_attr(out, node, src, "path", "name");
        // Fully-qualified expression paths (`crate::diff::git::apply(...)`) are
        // inline usage edges, not module-level coupling.
        if let Some(ctx) = imports::qualified_path(
            node,
            src,
            module_name,
            scope.last().map(|s| s.name.as_str()),
        ) {
            out.symbols.imports.push(ctx);
        }
    }

    // Macro bodies are token trees, not parsed expressions — `m::f(…)` inside
    // `vec![…]`/`assert!(…)` would otherwise lose its usage credit. Scan the
    // raw token sequence for `ident :: ident` and credit it the same way.
    if kind == "token_tree" {
        let mut cursor = node.walk();
        let children: Vec<Node> = node.children(&mut cursor).collect();
        for w in children.windows(3) {
            if w[0].kind() == "identifier" && w[1].kind() == "::" && w[2].kind() == "identifier" {
                if let (Ok(base), Ok(item)) = (w[0].utf8_text(src), w[2].utf8_text(src)) {
                    record_attr(&mut out.usage.attribute_accesses, base, item);
                }
            }
        }
    }

    record_declarations(node, src, kind, scope, out);

    if token_map::is_leaf(kind) {
        return;
    }

    let opened = token_map::is_scope(kind);
    if opened {
        scope.push(Scope {
            name: symbols::scope_name(node, src),
            is_class: token_map::is_class(kind),
        });
    }
    let opened_fn = matches!(kind, "function_item" | "closure_expression");
    if opened_fn {
        fn_bounds.push(scope::bound_names(node, src));
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        visit(child, src, file_id, module_name, scope, fn_bounds, out);
    }

    if opened_fn {
        fn_bounds.pop();
    }
    if opened {
        scope.pop();
    }
}

/// Record top-level `pub` declarations and `pub` methods in impl/trait blocks.
fn record_declarations(node: Node, src: &[u8], kind: &str, scope: &[Scope], out: &mut Walked) {
    if kind == "function_item" && scope.last().is_some_and(|s| s.is_class) {
        if symbols::is_pub(node) {
            if let Some(name) = symbols::def_name(node, src) {
                register_method(out, name, node.start_position().row + 1);
            }
        }
        return;
    }

    if !scope.is_empty() || !symbols::is_pub(node) {
        return;
    }
    let Some(dkind) = symbols::declared_kind(kind) else {
        return;
    };
    if let Some(name) = symbols::def_name(node, src) {
        declare(out, name, dkind, node.start_position().row + 1);
    }
}
