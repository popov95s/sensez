//! Pre-order AST traversal: emits the genericized token stream + spans, and
//! extracts imports/declarations while tracking an explicit lexical scope stack.

use super::{imports, lexeme, scope, symbols, tokens as token_map, typehints, units};
use crate::profiles::walk::{
    self, credit_attr, credit_name, declare, emit_mapped, register_method, Scope,
};
use crate::spine::ir::ImportPhase;
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

    // Imports are extracted, never descended into (keeps the token stream clean).
    if token_map::is_import(kind) {
        let enclosing = scope.last().map(|s| s.name.as_str());
        let phase = if is_under_type_checking_guard(node, src) {
            ImportPhase::TypeOnly
        } else {
            ImportPhase::Runtime
        };
        out.symbols
            .imports
            .extend(imports::extract(node, src, module_name, enclosing, phase));
        return;
    }

    // Docstrings (and any bare string statement) are documentation, not logic.
    // Under `eyez`, capture them for docs search before skipping token emission.
    if matches!(kind, "string" | "concatenated_string")
        && node
            .parent()
            .is_some_and(|p| p.kind() == "expression_statement")
    {
        #[cfg(feature = "eyez")]
        if crate::eyez::capture::python::is_docstring(node) {
            let scope_path: Vec<&str> = scope.iter().map(|s| s.name.as_str()).collect();
            crate::eyez::capture::python::push_docstring(out, module_name, &scope_path, node, src);
        }
        return;
    }

    // Comments never map to a structural token (`map_kind` returns `None`), so
    // capturing their text here cannot affect duplication either.
    #[cfg(feature = "eyez")]
    if kind == "comment" {
        let scope_path: Vec<&str> = scope.iter().map(|s| s.name.as_str()).collect();
        crate::eyez::capture::python::push_comment(out, module_name, &scope_path, node, src);
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

    // Count identifier occurrences and `obj.attr` accesses per base identifier,
    // so `crud.fetch(...)` can be credited to the module `crud` was bound to.
    if kind == "identifier" {
        credit_name(out, node, src);
    }
    if kind == "attribute" {
        credit_attr(out, node, src, "object", "attribute");
    }

    // f-string interpolations hold real expressions. Credit identifier and
    // attribute usage inside them so a function called only from an f-string
    // isn't flagged dead — but emit no tokens (string internals must stay out
    // of the duplication stream, and `is_leaf` still stops normal descent).
    if kind == "string" || kind == "concatenated_string" {
        credit_interpolations(node, src, out);
    }

    record_declarations(node, src, kind, scope, out);

    // Per-unit structural summaries for the design-smell pillar. `scope` still
    // reflects the *enclosing* scope here (the current node is pushed below),
    // so a function directly inside a class is a method.
    if kind == "function_definition" {
        let is_method = scope.last().is_some_and(|s| s.is_class);
        let mut unit = units::analyze_function(node, src, is_method);
        if let Some(enclosing) = scope.last().filter(|s| !s.is_class) {
            unit.is_nested = true;
            unit.parent = enclosing.name.clone();
        }
        out.units.functions.push(unit);
    } else if kind == "class_definition" {
        out.units.classes.push(units::analyze_class(node, src));
    }

    // Type hints are collected inline; pruned subtrees contain no annotations.
    match kind {
        "function_definition" => typehints::record_function(node, src, &mut out.units.type_hints),
        "assignment" => typehints::record_assignment(node, src, &mut out.units.type_hints),
        _ => {}
    }

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
            is_class: kind == "class_definition",
        });
    }
    // A function opens a new local-variable scope for lexeme normalization.
    let opened_fn = kind == "function_definition";
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

/// Usage-only mini-walk over a string's `interpolation` subtrees: bump
/// identifier counts and record attribute accesses, nothing else.
fn credit_interpolations(node: Node, src: &[u8], out: &mut Walked) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "identifier" => credit_name(out, child, src),
            "attribute" => {
                credit_attr(out, child, src, "object", "attribute");
                credit_interpolations(child, src, out);
            }
            _ => credit_interpolations(child, src, out),
        }
    }
}

fn is_under_type_checking_guard(node: Node, src: &[u8]) -> bool {
    let mut parent = node.parent();
    while let Some(ancestor) = parent {
        if ancestor.kind() == "if_statement"
            && ancestor
                .child_by_field_name("condition")
                .and_then(|n| n.utf8_text(src).ok())
                .is_some_and(is_type_checking_condition)
        {
            return true;
        }
        parent = ancestor.parent();
    }
    false
}

fn is_type_checking_condition(text: &str) -> bool {
    let condition = text.trim();
    condition == "TYPE_CHECKING" || condition.ends_with(".TYPE_CHECKING")
}

/// Record top-level declarations and class methods.
fn record_declarations(node: Node, src: &[u8], kind: &str, scope: &[Scope], out: &mut Walked) {
    // A function defined directly inside a class is a method.
    if kind == "function_definition" && scope.last().is_some_and(|s| s.is_class) {
        if let Some(name) = symbols::def_name(node, src) {
            register_method(out, name, node.start_position().row + 1);
        }
    }

    if !scope.is_empty() {
        return; // remaining records are module-level only
    }
    let line = node.start_position().row + 1;
    if token_map::is_scope(kind) {
        if let Some(name) = symbols::def_name(node, src) {
            let k = if kind == "class_definition" {
                SymbolKind::Class
            } else {
                SymbolKind::Function
            };
            declare(out, name, k, line);
        }
    } else if kind == "assignment" {
        for target in symbols::assignment_targets(node, src) {
            out.symbols
                .declared_kinds
                .entry(target.clone())
                .or_insert_with(|| SymbolKind::Variable);
            out.symbols
                .declared_lines
                .entry(target.clone())
                .or_insert(line);
            out.symbols.declared.push(target);
        }
        if let Some(all) = symbols::dunder_all(node, src) {
            out.symbols.dunder_all = Some(all);
        }
    } else if kind == "decorated_definition" {
        if let Some(def) = node.child_by_field_name("definition") {
            if let Some(name) = symbols::def_name(def, src) {
                out.symbols
                    .decorators
                    .insert(name, symbols::decorator_paths(node, src));
            }
        }
    }
}

#[cfg(test)]
#[path = "traversal_tests.rs"]
mod tests;
