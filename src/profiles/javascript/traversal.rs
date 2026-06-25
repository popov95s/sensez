//! Pre-order JS/TS traversal: emits the genericized token stream + spans and
//! extracts imports/declarations while tracking a lexical scope stack.

use super::{imports, lexeme, scope, symbols, tokens as token_map, typehints, units};
use crate::profiles::walk::{
    self, credit_attr, credit_name, declare, emit_mapped, register_method, Scope,
};
use crate::spine::ir::SymbolKind;
use crate::spine::ir::Walked;
use std::collections::HashSet;
use tree_sitter::Node;

/// Walk `root` pre-order, producing tokens/spans/imports for the file.
pub fn walk(root: Node, src: &[u8], file_id: u32, module_name: &str) -> Walked {
    walk::run_with_method_attrs(root, src, file_id, module_name, visit)
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

    // ES imports / re-exports: extracted, not descended into.
    if imports::is_import_statement(kind) {
        let enclosing = scope.last().map(|s| s.name.as_str());
        let ctxs = imports::extract(node, src, module_name, enclosing);
        if !ctxs.is_empty() {
            out.symbols.imports.extend(ctxs);
            return;
        }
    }

    // CommonJS / dynamic `require("…")` / `import("…")`: record an edge, but
    // keep emitting the call token so duplication still sees the call shape.
    if kind == "call_expression" {
        let enclosing = scope.last().map(|s| s.name.as_str());
        if let Some(ctx) = imports::require_import(node, src, module_name, enclosing) {
            out.symbols.imports.push(ctx);
        }
    }

    // Directive prologues / bare string statements ("use strict") are not logic.
    if kind == "string"
        && node
            .parent()
            .is_some_and(|p| p.kind() == "expression_statement")
    {
        return;
    }

    // Comments never map to a structural token, so capturing their text for the
    // eyez index cannot affect duplication.
    #[cfg(feature = "eyez")]
    if kind == "comment" {
        let scope_path: Vec<&str> = scope.iter().map(|s| s.name.as_str()).collect();
        crate::eyez::capture::javascript::push_comment(out, module_name, &scope_path, node, src);
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

    if kind == "identifier" {
        credit_name(out, node, src);
    }
    // `obj.attr` member access per base identifier (attribute-access crediting).
    if kind == "member_expression" {
        credit_attr(out, node, src, "object", "property");
    }

    record_declarations(node, src, kind, scope, out);
    record_units(node, src, kind, scope, out);

    if token_map::is_leaf(kind) {
        return;
    }

    let opened = token_map::is_scope(kind);
    if opened {
        let name = node
            .child_by_field_name("name")
            .and_then(|n| n.utf8_text(src).ok())
            .unwrap_or("<anon>")
            .to_string();
        scope.push(Scope {
            name,
            is_class: token_map::is_class(kind),
        });
    }
    let opened_fn = opened && !token_map::is_class(kind);
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

/// Per-unit structural summaries + type hints for the design-smell pillar.
/// `scope` still reflects the *enclosing* scope here (the current node is pushed
/// afterward), so a function directly inside a class is a method and a function
/// declaration inside another function is nested.
fn record_units(node: Node, src: &[u8], kind: &str, scope: &[Scope], out: &mut Walked) {
    if units::is_function(kind) {
        let is_method = scope.last().is_some_and(|s| s.is_class);
        let mut unit = units::analyze_function(node, src, is_method);
        // Only a *named* nested function declaration is the "untestable hidden
        // helper" smell; arrow/expression callbacks are idiomatic and excluded.
        let enclosed_by_fn = scope.last().is_some_and(|s| !s.is_class);
        if enclosed_by_fn
            && matches!(
                kind,
                "function_declaration" | "generator_function_declaration"
            )
        {
            unit.is_nested = true;
            unit.parent = scope.last().map(|s| s.name.clone()).unwrap_or_default();
        }
        out.units.functions.push(unit);
        typehints::record_function(node, src, &mut out.units.type_hints);
    } else if token_map::is_class(kind) {
        out.units
            .classes
            .push(super::classunit::analyze_class(node, src));
    } else if kind == "variable_declarator" {
        typehints::record_declaration(node, src, &mut out.units.type_hints);
    }
}

/// Record top-level declarations and class methods.
fn record_declarations(node: Node, src: &[u8], kind: &str, scope: &[Scope], out: &mut Walked) {
    if kind == "method_definition" && scope.last().is_some_and(|s| s.is_class) {
        if let Some(name) = symbols::def_name(node, src) {
            register_method(out, name, node.start_position().row + 1);
        }
    }

    if !scope.is_empty() {
        return;
    }
    let line = node.start_position().row + 1;
    match kind {
        "function_declaration" | "generator_function_declaration" => {
            if let Some(name) = symbols::def_name(node, src) {
                declare(out, name, SymbolKind::Function, line);
            }
        }
        "class_declaration" | "abstract_class_declaration" => {
            if let Some(name) = symbols::def_name(node, src) {
                declare(out, name, SymbolKind::Class, line);
            }
        }
        "lexical_declaration" | "variable_declaration" => {
            for (name, vkind) in symbols::declared_vars(node, src) {
                out.symbols
                    .declared_kinds
                    .entry(name.clone())
                    .or_insert(vkind);
                out.symbols
                    .declared_lines
                    .entry(name.clone())
                    .or_insert(line);
                out.symbols.declared.push(name);
            }
        }
        _ => {}
    }
}
