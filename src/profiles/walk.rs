//! Shared traversal building blocks used by every language walker.
//!
//! Each language owns its whole pre-order walk (traversal logic is coupled to
//! one grammar's node kinds), but the bookkeeping that targets the shared
//! [`Walked`] output is identical everywhere and lives here once.

use crate::spine::ir::tokens::{StructuralToken, TokenSpan};
use crate::spine::ir::{bump, record_attr, FunctionUnit, PerfLine, SymbolKind, Walked};
use std::collections::HashSet;
use tree_sitter::Node;

/// One frame of a walker's lexical scope stack.
pub(crate) struct Scope {
    pub(crate) name: String,
    pub(crate) is_class: bool,
}

/// A language's per-token lexeme-code function (see `lang::lexeme`).
pub(crate) type LexemeFn = fn(Node, StructuralToken, &[u8], &[HashSet<String>]) -> u64;

/// A language's recursive visit function (the per-grammar walk body).
pub(crate) type VisitFn =
    fn(Node, &[u8], u32, &str, &mut Vec<Scope>, &mut Vec<HashSet<String>>, &mut Walked);

/// A language's mutation-target resolver: the root identifier of a
/// subscript/member chain plus whether the path crossed a member/attribute
/// access (the node kinds and field names are per-grammar).
pub(crate) type TargetRootFn = fn(Node, &[u8]) -> Option<(String, bool)>;

/// Drive one file's walk: set up the scope/bound-name stacks, run the
/// language's `visit` from the root, and return the populated [`Walked`].
pub(crate) fn run(
    root: Node,
    src: &[u8],
    file_id: u32,
    module_name: &str,
    visit: VisitFn,
) -> Walked {
    let mut out = Walked::default();
    let mut scope: Vec<Scope> = Vec::new();
    let mut fn_bounds: Vec<HashSet<String>> = Vec::new();
    visit(
        root,
        src,
        file_id,
        module_name,
        &mut scope,
        &mut fn_bounds,
        &mut out,
    );
    out
}

/// [`run`] followed by the [`attach_method_attrs`] post-pass — the exact walk
/// entry the Python and JS/TS profiles share (Rust has no method-attr pass).
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

/// Count one identifier occurrence (intra-module usage analysis). Allocates a
/// key only on first sight — repeat occurrences just bump the counter.
pub(crate) fn credit_name(out: &mut Walked, node: Node, src: &[u8]) {
    if let Ok(text) = node.utf8_text(src) {
        bump(&mut out.usage.name_counts, text);
    }
}

/// Record member access on a plain-identifier base (`obj.attr`, `mod::item`)
/// so usage can be credited to whatever the base name was bound to. The field
/// names are per-grammar (`object`/`attribute`, `object`/`property`,
/// `value`/`field`, `path`/`name`).
pub(crate) fn credit_attr(
    out: &mut Walked,
    node: Node,
    src: &[u8],
    base_field: &str,
    attr_field: &str,
) {
    if let (Some(attr), Some(base)) = (
        node.child_by_field_name(attr_field)
            .and_then(|n| n.utf8_text(src).ok()),
        node.child_by_field_name(base_field)
            .filter(|o| o.kind() == "identifier")
            .and_then(|o| o.utf8_text(src).ok()),
    ) {
        record_attr(&mut out.usage.attribute_accesses, base, attr);
    }
}

/// Map a node to its structural token (per-grammar `map_kind`) and, when it
/// is significant, emit it with its lexeme code (per-language `code`).
pub(crate) fn emit_mapped(
    out: &mut Walked,
    file_id: u32,
    node: Node,
    src: &[u8],
    fn_bounds: &[HashSet<String>],
    map_kind: fn(&str) -> Option<StructuralToken>,
    code: LexemeFn,
) {
    if let Some(tok) = map_kind(node.kind()) {
        emit(out, file_id, node, tok, code(node, tok, src, fn_bounds));
    }
}

/// Emit one structural token with its lexeme code and 1-indexed line span.
fn emit(out: &mut Walked, file_id: u32, node: Node, tok: StructuralToken, code: u64) {
    let start = node.start_position();
    let end = node.end_position();
    out.syntax.tokens.push(tok);
    out.syntax.lexemes.push(code);
    out.syntax.spans.push(TokenSpan {
        file_id,
        start_row: start.row + 1,
        end_row: end.row + 1,
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

/// Post-pass (Python/JS/TS): fill each class's `method_attr_use` (method name →
/// `self`/`this` attr set) from the `self_attrs` each method collected during
/// its own body walk — avoiding a second per-method walk. A method is
/// attributed to the innermost class enclosing its def line, and only when the
/// class lists it as a direct method (so a function defined inside an `if` in a
/// class body is correctly excluded).
pub(crate) fn attach_method_attrs(out: &mut Walked) {
    let assignments: Vec<(usize, String, HashSet<String>)> = out
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
                .map(|(i, _)| (i, f.name.clone(), f.self_attrs.clone()))
        })
        .collect();
    for (i, name, attrs) in assignments {
        out.units.classes[i].method_attr_use.insert(name, attrs);
    }
}

/// Route a mutation target to the right set: `attr_mutated_names` when the path
/// to the root identifier crossed a member/attribute access, else
/// `mutated_names`. `target_root` is the per-grammar resolver (it knows the
/// language's subscript/member node kinds and field names).
pub(crate) fn record_mutation_root(
    unit: &mut FunctionUnit,
    node: Node,
    src: &[u8],
    target_root: TargetRootFn,
) {
    if let Some((root, via_attr)) = target_root(node, src) {
        if via_attr {
            unit.attr_mutated_names.insert(root);
        } else {
            unit.mutated_names.insert(root);
        }
    }
}

/// Record a short string fallback literal occurrence.
pub(crate) fn record_short_string_fallback(
    unit: &mut FunctionUnit,
    literal_len: Option<usize>,
    line: usize,
) {
    if literal_len.is_some_and(|len| len <= 1) {
        unit.short_string_fallback_lines.push(line);
    }
}

pub(crate) fn node_text<'a>(node: Node, src: &'a [u8]) -> Option<&'a str> {
    node.utf8_text(src).ok()
}

pub(crate) fn perf_line(node: Node, src: &[u8], subject_fields: &[&str]) -> PerfLine {
    PerfLine {
        line: node.start_position().row + 1,
        subject: loop_subject(node, src, subject_fields).unwrap_or_default(),
    }
}

fn loop_subject(node: Node, src: &[u8], subject_fields: &[&str]) -> Option<String> {
    subject_fields
        .iter()
        .find_map(|field| node.child_by_field_name(field))
        .filter(|n| n.kind() == "identifier")
        .and_then(|n| node_text(n, src))
        .map(str::to_string)
}
