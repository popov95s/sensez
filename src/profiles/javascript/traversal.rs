//! Pre-order JS/TS traversal: emits the genericized token stream + spans and
//! extracts imports/declarations while tracking a lexical scope stack. Function
//! metric collection is fused into this single walk — there is no second
//! per-function body pass.

use super::comment_runs::CommentRun;
use super::typespace::TypeSpace;
use super::{
    imports, lexeme, namespaces, risk_facts, scope, symbols, tokens as token_map, typehints, units,
};
use crate::profiles::lexeme::BoundNames;
use crate::profiles::lineage::Lineage;
use crate::profiles::walk::{
    self, credit_attr, credit_name, credit_string, declare, register_method, Scope,
};
use crate::spine::ir::SymbolKind;
use crate::spine::ir::Walked;
use tree_sitter::Node;

struct ActiveFn {
    collector: units::FunctionFacts,
    index: usize,
    is_test_framework_callback: bool,
}

struct VisitCtx<'a, 'tree> {
    scope: &'a mut Vec<Scope>,
    fn_bounds: &'a mut Vec<BoundNames>,
    out: &'a mut Walked,
    active_fns: &'a mut Vec<ActiveFn>,
    /// Ancestors of the node being visited, root first.
    path: Vec<Node<'tree>>,
    free_reads: namespaces::FreeReads,
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
        path: Vec::new(),
        free_reads: namespaces::FreeReads::default(),
    };
    visit(
        root,
        src,
        file_id,
        module_name,
        false,
        TypeSpace::Value,
        &mut ctx,
    );
    let free_reads = ctx.free_reads;
    namespaces::discount_namespace_hops(&mut out);
    namespaces::narrow(&free_reads, &mut out.symbols.imports);
    crate::profiles::comments::attach(&mut out);
    walk::attach_method_attrs(&mut out);
    out
}

fn visit<'tree>(
    node: Node<'tree>,
    src: &[u8],
    file_id: u32,
    module_name: &str,
    is_member_property: bool,
    space: TypeSpace,
    ctx: &mut VisitCtx<'_, 'tree>,
) {
    let kind = node.kind();
    let parent = ctx.path.last().copied();

    // ES imports / re-exports: extracted, not descended into.
    if imports::is_import_statement(kind) {
        let enclosing = ctx.scope.last().map(|s| s.name.as_str());
        let ctxs = imports::extract(node, src, module_name, enclosing);
        if !ctxs.is_empty() {
            ctx.out.symbols.imports.extend(ctxs);
            ctx.free_reads.skipped(node, src);
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
        super::mock_targets::record(node, src, &mut ctx.out.units.mock_targets);
    }

    // Directive prologues / bare string statements ("use strict") are not logic.
    if kind == "string" && parent.is_some_and(|p| p.kind() == "expression_statement") {
        return;
    }

    // Comments never map to a structural token, so capturing their text for the
    // eyez index cannot affect duplication.
    if kind == "comment" {
        #[cfg(feature = "eyez")]
        {
            let scope_path: Vec<&str> = ctx.scope.iter().map(|s| s.name.as_str()).collect();
            let owner = leading_function(node, src);
            crate::eyez::capture::javascript::push_comment(
                ctx.out,
                module_name,
                &scope_path,
                owner.as_deref(),
                node,
                src,
            );
        }
        return;
    }

    if let Some(token) = token_map::map_kind(kind) {
        let code = lexeme::code(node, token, src, ctx.fn_bounds, is_member_property);
        if let Some(code) = space.code(code) {
            walk::emit(ctx.out, file_id, node, token, code);
        }
    }

    if matches!(
        kind,
        "identifier" | "shorthand_property_identifier" | "type_identifier"
    ) {
        credit_name(ctx.out, node, src);
        ctx.free_reads.record(node, &ctx.path, src);
    }
    if kind == "string" {
        if let Some(value) = symbols::quoted_string_value(node, src) {
            credit_string(ctx.out, value);
        }
    }
    if kind == "member_expression" {
        credit_attr(ctx.out, node, src, "object", "property");
    }
    if kind == "nested_type_identifier" {
        credit_attr(ctx.out, node, src, "module", "name");
    }

    record_declarations(node, src, kind, ctx);
    record_units(node, src, kind, ctx);

    if units::is_function(kind) {
        start_function(node, src, kind, ctx);
    }

    if token_map::is_leaf(kind) {
        return;
    }

    let opened = token_map::is_scope(kind);
    if opened {
        let name = symbols::function_name(Lineage::new(&ctx.path, node), src)
            .unwrap_or_else(|| crate::spine::ir::ANONYMOUS.to_string());
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
    ctx.path.push(node);
    let mut comments = CommentRun::default();
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        comments.push(ctx.out, child, node, src);
        let frame = ctx
            .active_fns
            .last_mut()
            .and_then(|active| active.collector.enter(Lineage::new(&ctx.path, child), src));
        visit(
            child,
            src,
            file_id,
            module_name,
            property_id == Some(child.id()),
            space.enter(node, child),
            ctx,
        );
        if let Some(f) = frame {
            ctx.active_fns.last_mut().unwrap().collector.leave(f);
        }
    }
    comments.flush(ctx.out, node, src);
    ctx.path.pop();

    if opened_fn {
        ctx.fn_bounds.pop();
        let active = ctx.active_fns.pop().unwrap();
        ctx.out.units.functions[active.index] = active.collector.finish();
    }
    if opened {
        ctx.scope.pop();
    }
}

// Kept out of `visit` so the large collector never sits in each recursive frame.
#[inline(never)]
fn start_function<'tree>(node: Node<'tree>, src: &[u8], kind: &str, ctx: &mut VisitCtx<'_, 'tree>) {
    let lineage = Lineage::new(&ctx.path, node);
    let is_method = ctx.scope.last().is_some_and(|s| s.is_class);
    let mut collector = units::FunctionFacts::start(lineage, src, is_method);
    let parent = ctx.scope.last().filter(|s| !s.is_class);
    if parent.is_some_and(|parent| {
        super::local_functions::is_named_local_function(lineage, kind, &parent.name)
    }) {
        collector.unit.is_nested = true;
        collector.unit.parent = ctx.scope.last().map(|s| s.name.clone()).unwrap_or_default();
        collector.unit.parent_is_test_callback = ctx
            .active_fns
            .last()
            .is_some_and(|parent| parent.is_test_framework_callback);
        if let Some(parent_bounds) = ctx.fn_bounds.last() {
            let own = scope::bound_names(node, src);
            collector.outer_names = parent_bounds.difference(&own).copied().collect();
        }
    }
    let index = ctx.out.units.functions.len();
    ctx.out.units.functions.push(Default::default());
    typehints::record_function(lineage, src, &mut ctx.out.units.type_hints);
    ctx.active_fns.push(ActiveFn {
        collector,
        index,
        is_test_framework_callback: risk_facts::is_test_framework_callback(lineage, src),
    });
}

#[cfg(feature = "eyez")]
fn leading_function(node: Node, src: &[u8]) -> Option<String> {
    let next = std::iter::successors(node.next_named_sibling(), |n| n.next_named_sibling())
        .find(|n| n.kind() != "comment")?;
    let declaration = if next.kind() == "export_statement" {
        next.named_child(0)?
    } else {
        next
    };
    match declaration.kind() {
        "function_declaration" | "generator_function_declaration" | "method_definition" => {
            declaration
                .child_by_field_name("name")?
                .utf8_text(src)
                .ok()
                .map(str::to_owned)
        }
        _ => None,
    }
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
                record_default_export(node, &name, ctx);
                declare(ctx.out, name, SymbolKind::Function, line);
            }
        }
        "class_declaration" | "abstract_class_declaration" => {
            if let Some(name) = symbols::def_name(node, src) {
                record_default_export(node, &name, ctx);
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
            if node
                .parent()
                .is_some_and(|p| p.kind() == "export_statement")
            {
                for (name, callee) in symbols::initializer_callees(node, src) {
                    ctx.out
                        .symbols
                        .decorators
                        .entry(name)
                        .or_default()
                        .push(callee);
                }
            }
        }
        _ => {}
    }
}

fn record_default_export(node: Node, name: &str, ctx: &mut VisitCtx) {
    if symbols::is_default_export(node) {
        ctx.out.symbols.default_export = Some(name.to_string());
    }
}
