//! Shared traversal building blocks used by every language walker.

pub(crate) use super::node_query::{
    chain_hit, fed_by_call, identifiers, is_field_child, last_segment, loop_subject, node_text,
    perf_line, ChainShape,
};
use crate::profiles::comment_class::{self, CommentSyntax};
use crate::profiles::comments;
use crate::profiles::lexeme::BoundNames;
use crate::spine::ir::tokens::{StructuralToken, TokenSpan};
use crate::spine::ir::{
    bump, record_attr, CommentSpan, FallbackSite, FunctionUnit, StringFallback, SymbolKind, Walked,
};
use std::collections::HashSet;
use tree_sitter::Node;

/// One frame of a walker's lexical scope stack.
pub(crate) struct Scope {
    pub(crate) name: String,
    pub(crate) is_class: bool,
}

/// A language's per-token lexeme-code function (see `lang::lexeme`).
pub(crate) type LexemeFn = fn(Node, StructuralToken, &[u8], &[BoundNames]) -> u64;

/// A language's recursive visit function (the per-grammar walk body).
pub(crate) type VisitFn =
    fn(Node, &[u8], u32, &str, &mut Vec<Scope>, &mut Vec<BoundNames>, &mut Walked);

/// Root identifier of a mutated subscript/member chain.
pub(crate) enum MutationTarget {
    /// `d["a"]["b"]`: the root itself is mutated.
    Direct(String),
    /// `m.items.push(...)`: the path crossed a member/attribute access.
    ViaAttribute(String),
}

impl MutationTarget {
    pub(crate) fn via_attribute(self) -> Self {
        match self {
            MutationTarget::Direct(root) | MutationTarget::ViaAttribute(root) => {
                MutationTarget::ViaAttribute(root)
            }
        }
    }
}

/// A language's mutation-target resolver (node kinds and field names are per-grammar).
pub(crate) type TargetRootFn = fn(Node, &[u8]) -> Option<MutationTarget>;

/// Run the language's `visit` over one file and return the populated [`Walked`].
pub(crate) fn run(
    root: Node,
    src: &[u8],
    file_id: u32,
    module_name: &str,
    visit: VisitFn,
) -> Walked {
    let mut out = Walked::default();
    let mut scope: Vec<Scope> = Vec::new();
    let mut fn_bounds: Vec<BoundNames> = Vec::new();
    visit(
        root,
        src,
        file_id,
        module_name,
        &mut scope,
        &mut fn_bounds,
        &mut out,
    );
    comments::attach(&mut out);
    out
}

/// [`run`] followed by the [`attach_method_attrs`] post-pass.
pub(crate) fn run_with_method_attrs(
    root: Node,
    src: &[u8],
    file_id: u32,
    module_name: &str,
    visit: VisitFn,
) -> Walked {
    let mut out = run(root, src, file_id, module_name, visit);
    attach_method_attrs(&mut out);
    out
}

/// Count one identifier occurrence for intra-module usage.
pub(crate) fn credit_name(out: &mut Walked, node: Node, src: &[u8]) {
    if let Ok(text) = node.utf8_text(src) {
        bump(&mut out.usage.name_counts, text);
    }
}

/// Record member access `base.attr` by identifier base, else the leaf attribute name.
pub(crate) fn credit_attr(
    out: &mut Walked,
    node: Node,
    src: &[u8],
    base_field: &str,
    attr_field: &str,
) {
    let Some(attr) = named_child_text(node, src, attr_field) else {
        return;
    };
    if let Some(base) = ident_child_text(node, src, base_field) {
        record_attr(&mut out.usage.attribute_accesses, base, attr);
        return;
    }
    if !out.usage.chained_attribute_names.contains(attr) {
        out.usage.chained_attribute_names.insert(attr.to_string());
    }
}

/// [`credit_attr`] that also records non-identifier base paths (`self.model.x`).
pub(crate) fn credit_attr_with_base_path(
    out: &mut Walked,
    node: Node,
    src: &[u8],
    base_field: &str,
    attr_field: &str,
) {
    let base_path = match ident_child_text(node, src, base_field) {
        Some(_) => None,
        None => named_child_text(node, src, base_field),
    };
    if let (Some(base), Some(attr)) = (base_path, named_child_text(node, src, attr_field)) {
        record_attr(&mut out.usage.attribute_path_accesses, base, attr);
    }
    credit_attr(out, node, src, base_field, attr_field);
}

fn named_child_text<'a>(node: Node<'a>, src: &'a [u8], field: &str) -> Option<&'a str> {
    node.child_by_field_name(field)
        .and_then(|n| n.utf8_text(src).ok())
}

fn ident_child_text<'a>(node: Node<'a>, src: &'a [u8], field: &str) -> Option<&'a str> {
    let child = node.child_by_field_name(field)?;
    if child.kind() != "identifier" {
        return None;
    }
    child.utf8_text(src).ok()
}

pub(crate) fn credit_string(out: &mut Walked, value: &str) {
    if !value.is_empty() && !out.usage.string_literals.contains(value) {
        out.usage.string_literals.insert(value.to_string());
    }
}

/// Emit `node` with its lexeme code when `map_kind` gives it a structural token.
pub(crate) fn emit_mapped(
    out: &mut Walked,
    file_id: u32,
    node: Node,
    src: &[u8],
    fn_bounds: &[BoundNames],
    map_kind: fn(&str) -> Option<StructuralToken>,
    code: LexemeFn,
) {
    if let Some(tok) = map_kind(node.kind()) {
        emit(out, file_id, node, tok, code(node, tok, src, fn_bounds));
    }
}

/// Emit one structural token with its lexeme code and 1-indexed line span.
pub(crate) fn emit(out: &mut Walked, file_id: u32, node: Node, tok: StructuralToken, code: u64) {
    let start = node.start_position();
    let end = node.end_position();
    out.syntax.tokens.push(tok);
    out.syntax.lexemes.push(code);
    out.syntax.spans.push(TokenSpan {
        file_id,
        start_row: (start.row + 1) as u32,
        end_row: (end.row + 1) as u32,
        start_byte: node.start_byte() as u32,
        end_byte: node.end_byte() as u32,
    });
}

/// Record one top-level declaration (name, kind, definition line).
pub(crate) fn declare(out: &mut Walked, name: String, kind: SymbolKind, line: usize) {
    out.symbols.declared_kinds.insert(name.clone(), kind);
    out.symbols.declared_lines.insert(name.clone(), line);
    out.symbols.declared.push(name);
}

pub(crate) fn register_method(out: &mut Walked, name: String, line: usize) {
    out.symbols.methods.push((name, line));
}

/// Fill each class's `method_attr_use` and `method_calls` from its direct methods' units.
pub(crate) fn attach_method_attrs(out: &mut Walked) {
    let assignments: Vec<(usize, String, HashSet<String>, HashSet<String>)> = out
        .units
        .functions
        .iter()
        .filter(|f| f.is_method)
        .filter_map(|f| {
            out.units
                .classes
                .iter()
                .enumerate()
                .filter(|(_, c)| c.start_line <= f.start_line && f.end_line <= c.end_line)
                .max_by_key(|(_, c)| c.start_line)
                .filter(|(_, c)| c.methods.contains(&f.name))
                .map(|(i, _)| {
                    (
                        i,
                        f.name.clone(),
                        f.self_attrs.clone(),
                        f.own_method_calls.clone(),
                    )
                })
        })
        .collect();
    for (i, name, attrs, calls) in assignments {
        out.units.classes[i]
            .method_attr_use
            .insert(name.clone(), attrs);
        out.units.classes[i].method_calls.insert(name, calls);
    }
}

/// Record the root of a mutated chain in `attr_mutated_names` or `mutated_names`.
pub(crate) fn record_mutation_root(
    unit: &mut FunctionUnit,
    node: Node,
    src: &[u8],
    target_root: TargetRootFn,
) {
    match target_root(node, src) {
        Some(MutationTarget::ViaAttribute(root)) => {
            unit.attr_mutated_names.insert(root);
        }
        Some(MutationTarget::Direct(root)) => {
            unit.mutated_names.insert(root);
        }
        None => {}
    }
}

/// Remember the first line `recv` was subscripted with a string-literal key.
pub(crate) fn record_str_key_line(unit: &mut FunctionUnit, recv: &str, line: usize) {
    unit.str_key_lines.entry(recv.to_string()).or_insert(line);
}

/// Record a string-literal fallback; non-literal fallbacks are ignored.
pub(crate) fn record_string_fallback(
    unit: &mut FunctionUnit,
    literal: Option<String>,
    site: FallbackSite,
    line: usize,
) {
    if let Some(literal) = literal {
        unit.string_fallbacks.push(StringFallback {
            line,
            literal,
            sink: site.sink,
            target: site.target,
        });
    }
}

/// Record a classified source comment span for shared post-walk attachment.
pub(crate) fn record_comment_span(
    out: &mut Walked,
    node: Node,
    parent: Option<Node>,
    src: &[u8],
    syntax: &CommentSyntax,
) {
    out.units.comment_spans.push(CommentSpan {
        start_line: node.start_position().row + 1,
        end_line: node.end_position().row + 1,
        class: comment_class::classify(node, parent, src, syntax),
    });
}

/// Record each comment of `block`, a run of adjacent comment children of `parent`.
pub(crate) fn record_comment_block(
    out: &mut Walked,
    block: &[Node],
    parent: Node,
    src: &[u8],
    syntax: &CommentSyntax,
) {
    out.units
        .comment_spans
        .extend(block.iter().map(|node| CommentSpan {
            start_line: node.start_position().row + 1,
            end_line: node.end_position().row + 1,
            class: comment_class::classify_in(*node, block, Some(parent), src, syntax),
        }));
}
