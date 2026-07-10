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
use crate::config::smells::{Smells, Strictness};
use crate::profiles::typevocab::{
    base_type, is_bool_type, is_primitive_scalar_alias, loose_kind, LooseTypeKind,
};
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
            loose_typing(ctx, m, cfg, out);
        }
        if cfg.magic_string_default && !m.short_string_fallback_lines.is_empty() {
            magic_string_default(ctx, m, out);
        }
        if cfg.tuple_packing {
            tuple_packing(ctx, m, cfg, out);
        }
    }
    if cfg.loose_typing && cfg.loose_typing_strictness == Strictness::High {
        type_hiding_aliases(ctx, out);
    }
}

/// One finding per function listing every loosely-typed param (and the return).
fn loose_typing(
    ctx: &SmellContext<'_>,
    m: &FunctionMetrics,
    cfg: &Smells,
    out: &mut Vec<SmellFinding>,
) {
    let (offenders, param_escape) =
        m.param_names
            .iter()
            .fold((Vec::new(), false), |(mut labels, escape), p| {
                if matches!(p.as_str(), "self" | "cls" | "args" | "kwargs") {
                    return (labels, escape);
                }
                let next = ctx
                    .type_hints
                    .param_types
                    .get(&(m.name.clone(), p.clone()))
                    .and_then(|ty| {
                        reportable_loose(ctx, ty, cfg.loose_typing_strictness)
                            .map(|kind| (format!("{p}: {ty}"), kind))
                    });
                if let Some((label, kind)) = next {
                    labels.push(label);
                    (labels, escape || kind == LooseTypeKind::EscapeHatch)
                } else {
                    (labels, escape)
                }
            });
    // Tuple returns are tuple_packing's finding — don't double-report here.
    let ret = ctx
        .type_hints
        .return_types
        .get(&m.name)
        .and_then(|ty| reportable_return(ctx, ty, cfg.loose_typing_strictness));
    if offenders.is_empty() && ret.is_none() {
        return;
    }
    let mut parts = Vec::new();
    if !offenders.is_empty() {
        parts.push(format!("params [{}]", offenders.join(", ")));
    }
    if let Some((ty, _)) = ret {
        parts.push(format!("returns {ty}"));
    }
    let any = param_escape || ret.is_some_and(|(_, kind)| kind == LooseTypeKind::EscapeHatch);
    let severity = if any {
        Severity::Warning
    } else {
        Severity::Info
    };
    out.push(make(
        SmellKind::LooseTyping,
        format!(
            "{} — replace loose types with {}; do not silence this with a shallow type alias",
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

fn reportable_return<'a>(
    ctx: &SmellContext<'_>,
    ty: &'a str,
    strictness: Strictness,
) -> Option<(&'a str, LooseTypeKind)> {
    if matches!(base_type(ty), "tuple" | "Tuple") {
        return None;
    }
    reportable_loose(ctx, ty, strictness).map(|kind| (ty, kind))
}

fn reportable_loose(
    ctx: &SmellContext<'_>,
    annotation: &str,
    strictness: Strictness,
) -> Option<LooseTypeKind> {
    let kind = loose_kind(ctx.language, annotation)?;
    match (strictness, kind) {
        (Strictness::Low, LooseTypeKind::EscapeHatch)
        | (Strictness::Medium, LooseTypeKind::EscapeHatch)
        | (Strictness::Medium, LooseTypeKind::SchemaErasing)
        | (Strictness::High, _) => Some(kind),
        _ => None,
    }
}

fn type_hiding_aliases(ctx: &SmellContext<'_>, out: &mut Vec<SmellFinding>) {
    for alias in &ctx.type_hints.type_aliases {
        let loose = reportable_loose(ctx, &alias.target, Strictness::High).is_some();
        if !loose && !is_primitive_scalar_alias(ctx.language, &alias.target) {
            continue;
        }
        out.push(make(
            SmellKind::LooseTyping,
            format!(
                "type alias {} = {} hides a loose type-safety contract — replace it with {} instead of a shallow alias",
                alias.name,
                alias.target,
                structure_target(ctx.language)
            ),
            ctx.path,
            alias.line,
            &alias.name,
            Severity::Warning,
            1,
            0,
        ));
    }
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
    let mut depth = BracketDepth::default();
    let mut count = 1usize;
    for c in body.chars() {
        match c {
            '[' | '(' => depth.open(),
            ']' | ')' => depth.close(),
            ',' if depth.is_top_level() => count += 1,
            _ => {}
        }
    }
    count
}

#[derive(Default)]
struct BracketDepth {
    value: usize,
}

impl BracketDepth {
    fn open(&mut self) {
        self.value += 1;
    }

    fn close(&mut self) {
        self.value = self.value.saturating_sub(1);
    }

    fn is_top_level(&self) -> bool {
        self.value == 0
    }
}
