//! Pre-order JS/TS traversal: emits the genericized token stream + spans and
//! extracts imports/declarations while tracking a lexical scope stack. Function
//! metric collection is fused into this single walk — there is no second
//! per-function body pass.

use super::{imports, lexeme, scope, symbols, tokens as token_map, typehints, units};
use crate::profiles::lexeme::BoundNames;
use crate::profiles::walk::{
    self, credit_attr, credit_name, credit_string, declare, register_method, Scope,
};
use crate::spine::ir::SymbolKind;
use crate::spine::ir::Walked;
use tree_sitter::Node;

struct ActiveFn {
    collector: units::FunctionFacts,
    index: usize,
}

struct VisitCtx<'a> {
    scope: &'a mut Vec<Scope>,
    fn_bounds: &'a mut Vec<BoundNames>,
    out: &'a mut Walked,
    active_fns: &'a mut Vec<ActiveFn>,
}

/// Walk `root` pre-order, producing tokens/spans/imports for the file.
pub fn walk(root: Node, src: &[u8], file_id: u32, module_name: &str) -> Walked {
    let mut out = Walked::default();
    let mut scope: Vec<Scope> = Vec::new();
    let mut fn_bounds: Vec<BoundNames> = Vec::new();
    let mut active_fns: Vec<ActiveFn> = Vec::new();
    let mut ctx = VisitCtx {
        scope: &mut scope,
        fn_bounds: &mut fn_bounds,
        out: &mut out,
        active_fns: &mut active_fns,
    };
    visit(root, src, file_id, module_name, false, &mut ctx);
    crate::profiles::comments::attach(&mut out);
    walk::attach_method_attrs(&mut out);
    out
}

fn visit(
    node: Node,
    src: &[u8],
    file_id: u32,
    module_name: &str,
    is_member_property: bool,
    ctx: &mut VisitCtx,
) {
    let kind = node.kind();

    // ES imports / re-exports: extracted, not descended into.
    if imports::is_import_statement(kind) {
        let enclosing = ctx.scope.last().map(|s| s.name.as_str());
        let ctxs = imports::extract(node, src, module_name, enclosing);
        if !ctxs.is_empty() {
            ctx.out.symbols.imports.extend(ctxs);
            return;
        }
    }

    // CommonJS / dynamic `require("…")` / `import("…")`: record an edge, but
    // keep emitting the call token so duplication still sees the call shape.
    if kind == "call_expression" {
        let enclosing = ctx.scope.last().map(|s| s.name.as_str());
        if let Some(ictx) = imports::require_import(node, src, module_name, enclosing) {
            ctx.out.symbols.imports.push(ictx);
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
    if kind == "comment" {
        walk::record_comment_span(ctx.out, node);
        #[cfg(feature = "eyez")]
        {
            let scope_path: Vec<&str> = ctx.scope.iter().map(|s| s.name.as_str()).collect();
            crate::eyez::capture::javascript::push_comment(
                ctx.out,
                module_name,
                &scope_path,
                node,
                src,
            );
        }
        return;
    }

    if let Some(token) = token_map::map_kind(kind) {
        let code = lexeme::code(node, token, src, ctx.fn_bounds, is_member_property);
        walk::emit(ctx.out, file_id, node, token, code);
    }

    if matches!(
        kind,
        "identifier" | "shorthand_property_identifier" | "type_identifier"
    ) {
        credit_name(ctx.out, node, src);
    }
    if kind == "string" {
        if let Some(value) = quoted_string_value(node, src) {
            credit_string(ctx.out, value);
        }
    }
    if kind == "member_expression" {
        credit_attr(ctx.out, node, src, "object", "property");
    }

    record_declarations(node, src, kind, ctx);
    record_units(node, src, kind, ctx);

    if units::is_function(kind) {
        let is_method = ctx.scope.last().is_some_and(|s| s.is_class);
        let mut collector = units::FunctionFacts::start(node, src, is_method);
        let enclosed_by_fn = ctx.scope.last().is_some_and(|s| !s.is_class);
        if enclosed_by_fn
            && matches!(
                kind,
                "function_declaration" | "generator_function_declaration"
            )
        {
            collector.unit.is_nested = true;
            collector.unit.parent = ctx.scope.last().map(|s| s.name.clone()).unwrap_or_default();
        }
        let index = ctx.out.units.functions.len();
        ctx.out.units.functions.push(Default::default());
        typehints::record_function(node, src, &mut ctx.out.units.type_hints);
        ctx.active_fns.push(ActiveFn { collector, index });
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
        ctx.scope.push(Scope {
            name,
            is_class: token_map::is_class(kind),
        });
    }
    let opened_fn = opened && !token_map::is_class(kind);
    if opened_fn {
        ctx.fn_bounds.push(scope::bound_names(node, src));
    }

    let property_id = (kind == "member_expression")
        .then(|| node.child_by_field_name("property").map(|child| child.id()))
        .flatten();
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        let frame = ctx
            .active_fns
            .last_mut()
            .and_then(|active| active.collector.enter(child, src));
        visit(
            child,
            src,
            file_id,
            module_name,
            property_id == Some(child.id()),
            ctx,
        );
        if let Some(f) = frame {
            ctx.active_fns.last_mut().unwrap().collector.leave(f);
        }
    }

    if opened_fn {
        ctx.fn_bounds.pop();
        let active = ctx.active_fns.pop().unwrap();
        ctx.out.units.functions[active.index] = active.collector.finish();
    }
    if opened {
        ctx.scope.pop();
    }
}

fn quoted_string_value<'a>(node: Node, src: &'a [u8]) -> Option<&'a str> {
    let text = node.utf8_text(src).ok()?.trim();
    let quote = text.chars().next().filter(|ch| matches!(ch, '"' | '\''))?;
    let body = text.get(quote.len_utf8()..)?;
    Some(body.strip_suffix(quote).unwrap_or(body))
}

/// Per-unit structural summaries for the design-smell pillar. Function-metric
/// collection is fused into the main traversal; this helper handles classes,
/// variable declarators, and type aliases only.
fn record_units(node: Node, src: &[u8], kind: &str, ctx: &mut VisitCtx) {
    if token_map::is_class(kind) {
        ctx.out
            .units
            .classes
            .push(super::classunit::analyze_class(node, src));
    } else if kind == "variable_declarator" {
        typehints::record_declaration(node, src, &mut ctx.out.units.type_hints);
    } else if kind == "type_alias_declaration" && ctx.scope.is_empty() {
        typehints::record_type_alias(node, src, &mut ctx.out.units.type_hints);
    }
}

/// Record top-level declarations and class methods.
fn record_declarations(node: Node, src: &[u8], kind: &str, ctx: &mut VisitCtx) {
    if kind == "method_definition" && ctx.scope.last().is_some_and(|s| s.is_class) {
        if let Some(name) = symbols::def_name(node, src) {
            register_method(ctx.out, name, node.start_position().row + 1);
        }
    }

    if !ctx.scope.is_empty() {
        return;
    }
    let line = node.start_position().row + 1;
    match kind {
        "function_declaration" | "generator_function_declaration" => {
            if let Some(name) = symbols::def_name(node, src) {
                declare(ctx.out, name, SymbolKind::Function, line);
            }
        }
        "class_declaration" | "abstract_class_declaration" => {
            if let Some(name) = symbols::def_name(node, src) {
                declare(ctx.out, name, SymbolKind::Class, line);
            }
        }
        "lexical_declaration" | "variable_declaration" => {
            for (name, vkind) in symbols::declared_vars(node, src) {
                ctx.out
                    .symbols
                    .declared_kinds
                    .entry(name.clone())
                    .or_insert(vkind);
                ctx.out
                    .symbols
                    .declared_lines
                    .entry(name.clone())
                    .or_insert(line);
                ctx.out.symbols.declared.push(name);
            }
        }
        _ => {}
    }
}
