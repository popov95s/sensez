//! Type-discipline smells (annotation-gated: unannotated code is skipped).
//!
//! - `loose_typing`: params/return typed as bare collections or `Any` where a
//!   dataclass/model almost always belongs.
//! - `magic_string_default`: fallback string literals (`or ""`, `|| "?"`,
//!   `cond ? value : "?"`, or any other empty / 1-char sentinel) that hide
//!   optionality behind a mandatory string contract.
//! - `boolean_blindness`: more than `max_bool_params` bool parameters.
//! - `tuple_packing`: position-based grouped data in returns.

use super::{grouped_value_target, make, structure_target, SmellContext};
use crate::config::smells::Smells;
use crate::profiles::typevocab::{base_type, is_bool_type, is_loose};
use crate::report::{Severity, SmellFinding, SmellKind};
use crate::spine::ir::FunctionMetrics;

pub fn detect(
    ctx: &SmellContext<'_>,
    metrics: &[FunctionMetrics],
    cfg: &Smells,
    out: &mut Vec<SmellFinding>,
) {
    for m in metrics {
        boolean_blindness(ctx, m, cfg, out);

        if cfg.loose_typing {
            loose_typing(ctx, m, out);
        }
        if cfg.magic_string_default && !m.short_string_fallback_lines.is_empty() {
            magic_string_default(ctx, m, out);
        }
        if cfg.tuple_packing {
            tuple_packing(ctx, m, cfg, out);
        }
    }
}

/// One finding per function listing every loosely-typed param (and the return).
fn loose_typing(ctx: &SmellContext<'_>, m: &FunctionMetrics, out: &mut Vec<SmellFinding>) {
    let mut offenders: Vec<String> = Vec::new();
    for p in &m.param_names {
        if matches!(p.as_str(), "self" | "cls" | "args" | "kwargs") {
            continue;
        }
        if let Some(ty) = ctx.type_hints.param_types.get(&(m.name.clone(), p.clone())) {
            if is_loose(ctx.language, ty) {
                offenders.push(format!("{p}: {ty}"));
            }
        }
    }
    // Tuple returns are tuple_packing's finding — don't double-report here.
    let ret = ctx
        .type_hints
        .return_types
        .get(&m.name)
        .filter(|ty| is_loose(ctx.language, ty) && !matches!(base_type(ty), "tuple" | "Tuple"));
    if offenders.is_empty() && ret.is_none() {
        return;
    }
    let mut parts = Vec::new();
    if !offenders.is_empty() {
        parts.push(format!("params [{}]", offenders.join(", ")));
    }
    if let Some(ty) = ret {
        parts.push(format!("returns {ty}"));
    }
    let any =
        offenders.iter().any(|o| is_escape_hatch(o)) || ret.is_some_and(|ty| is_escape_hatch(ty));
    let severity = if any {
        Severity::Warning
    } else {
        Severity::Info
    };
    out.push(make(
        SmellKind::LooseTyping,
        format!(
            "{} — replace loose collections with {}",
            parts.join("; "),
            structure_target(ctx.language)
        ),
        ctx.path,
        m.start_line,
        &m.name,
        severity,
        (offenders.len() + ret.is_some() as usize) as u32,
        0,
    ));
}

/// Fallback string literals used to paper over optional/nullable values.
fn magic_string_default(ctx: &SmellContext<'_>, m: &FunctionMetrics, out: &mut Vec<SmellFinding>) {
    let line = m
        .short_string_fallback_lines
        .first()
        .copied()
        .unwrap_or(m.start_line);
    let count = m.short_string_fallback_lines.len();
    out.push(make(
        SmellKind::MagicStringDefault,
        format!(
            "{} fallback string literal(s) (`or \"\"` / `|| \"?\"` / other 0-1 char sentinels) — hidden sentinel values; prefer a tighter contract with an optional string value or a dedicated default",
            count
        ),
        ctx.path,
        line,
        &m.name,
        Severity::Warning,
        count as u32,
        0,
    ));
}

/// More than `max_bool_params` bool-annotated parameters.
fn boolean_blindness(
    ctx: &SmellContext<'_>,
    m: &FunctionMetrics,
    cfg: &Smells,
    out: &mut Vec<SmellFinding>,
) {
    let bools = m
        .param_names
        .iter()
        .filter_map(|p| ctx.type_hints.param_types.get(&(m.name.clone(), p.clone())))
        .filter(|ty| is_bool_type(ctx.language, ty))
        .count();
    if bools > cfg.max_bool_params {
        out.push(make(
            SmellKind::BooleanBlindness,
            format!("{bools} boolean parameters — call sites are unreadable; consider an Enum or splitting the function"),
            ctx.path,
            m.start_line,
            &m.name,
            Severity::Warning,
            bools as u32,
            cfg.max_bool_params as u32,
        ));
    }
}

/// Bare tuple returns (`return a, b, c`) or `tuple[...]` return annotations
/// wider than `max_tuple_return` — position-based grouped data.
fn tuple_packing(
    ctx: &SmellContext<'_>,
    m: &FunctionMetrics,
    cfg: &Smells,
    out: &mut Vec<SmellFinding>,
) {
    let annotated = ctx
        .type_hints
        .return_types
        .get(&m.name)
        .filter(|ty| matches!(base_type(ty), "tuple" | "Tuple"))
        .map(|ty| tuple_arity(ty))
        .unwrap_or(0);
    let arity = m.max_tuple_return.max(annotated);
    if arity > cfg.max_tuple_return {
        out.push(make(
            SmellKind::TuplePacking,
            format!(
                "returns a {arity}-element tuple — positional grouped data; consider {}",
                grouped_value_target(ctx.language)
            ),
            ctx.path,
            m.start_line,
            &m.name,
            Severity::Info,
            arity as u32,
            cfg.max_tuple_return as u32,
        ));
    }
}

/// A loose annotation that is also an untyped *escape hatch* (`Any` / `any` /
/// `unknown`) rather than a bare collection — worth a Warning over an Info.
fn is_escape_hatch(annotation: &str) -> bool {
    annotation
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .any(|t| matches!(t, "Any" | "any" | "unknown"))
}

/// Top-level element count of `tuple[A, B, C]` (bracket-depth-aware).
fn tuple_arity(annotation: &str) -> usize {
    let body = annotation
        .split_once('[')
        .map(|(_, rest)| rest.strip_suffix(']').unwrap_or(rest))
        .unwrap_or("");
    if body.is_empty() {
        return 0;
    }
    let mut depth = 0usize;
    let mut count = 1usize;
    for c in body.chars() {
        match c {
            '[' | '(' => depth += 1,
            ']' | ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => count += 1,
            _ => {}
        }
    }
    count
}
