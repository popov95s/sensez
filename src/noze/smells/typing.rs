//! Type-discipline smells (annotation-gated: unannotated code is skipped).
//!
//! - `loose_typing`: params/return typed as bare collections or `Any` where a
//!   dataclass/model almost always belongs.
//! - `magic_string_default`: see [`super::magic_string`].
//! - `boolean_blindness`: more than `max_bool_params` bool parameters.
//! - `tuple_packing`: position-based grouped data in returns.

use super::{magic_string, make, SmellContext};
use crate::config::smells::{Smells, Strictness};
use crate::profiles::typevocab::{base_type, split_top_level, LooseTypeKind};
use crate::report::{Severity, SmellFinding, SmellKind};
use crate::spine::ir::FunctionUnit;

/// Widest same-primitive tuple still read as a vector, colour or range.
const MAX_HOMOGENEOUS_TUPLE: usize = 4;

pub fn detect(
    ctx: &SmellContext<'_>,
    metrics: &[FunctionUnit],
    cfg: &Smells,
    out: &mut Vec<SmellFinding>,
) {
    for m in metrics {
        boolean_blindness(ctx, m, cfg, out);

        if cfg.loose_typing {
            loose_typing(ctx, m, cfg, out);
        }
        if cfg.magic_string_default {
            magic_string::detect(ctx, m, cfg, out);
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
    m: &FunctionUnit,
    cfg: &Smells,
    out: &mut Vec<SmellFinding>,
) {
    let return_type = ctx.type_hints.return_type(m);
    let guarded = return_type.and_then(|ty| ctx.type_vocabulary.guarded_param(ty));
    let offenders: Vec<(String, LooseTypeKind)> = m
        .param_names
        .iter()
        .filter(|p| !matches!(p.as_str(), "self" | "cls" | "args" | "kwargs"))
        .filter(|p| !p.starts_with('_') && guarded != Some(p.as_str()))
        .filter(|p| !m.validated_names.contains(*p))
        .filter_map(|p| {
            let ty = ctx.type_hints.param_type(m, p)?;
            reportable_loose(ctx, ty, cfg.loose_typing_strictness)
                .map(|kind| (format!("{p}: {ty}"), kind))
        })
        .collect();
    // Tuple returns are tuple_packing's finding — don't double-report here.
    let ret = return_type.and_then(|ty| reportable_return(ctx, ty, cfg.loose_typing_strictness));
    if offenders.is_empty() && ret.is_none() {
        return;
    }
    let mut parts = Vec::new();
    if !offenders.is_empty() {
        let labels: Vec<&str> = offenders.iter().map(|(label, _)| label.as_str()).collect();
        parts.push(format!("params [{}]", labels.join(", ")));
    }
    if let Some((ty, _)) = ret {
        parts.push(format!("returns {ty}"));
    }
    let mut kinds = offenders
        .iter()
        .map(|(_, kind)| *kind)
        .chain(ret.map(|(_, kind)| kind));
    let any = kinds.clone().any(|kind| kind == LooseTypeKind::EscapeHatch);
    let severity = if any {
        Severity::Warning
    } else {
        Severity::Info
    };
    let advice = if kinds.all(|kind| kind == LooseTypeKind::SafeTop) {
        ctx.wording.safe_top_advice.to_string()
    } else {
        format!(
            "replace loose types with {}; do not silence this with a shallow type alias",
            ctx.wording.structure
        )
    };
    out.push(make(
        SmellKind::LooseTyping,
        format!("{} — {advice}", parts.join("; ")),
        ctx.path,
        m.start_line,
        &m.name,
        severity,
        (offenders.len() + ret.is_some() as usize) as u32,
        0,
    ));
}

/// More than `max_bool_params` bool-annotated parameters.
fn boolean_blindness(
    ctx: &SmellContext<'_>,
    m: &FunctionUnit,
    cfg: &Smells,
    out: &mut Vec<SmellFinding>,
) {
    let bools = m
        .param_names
        .iter()
        .filter_map(|p| ctx.type_hints.param_type(m, p))
        .filter(|ty| ctx.type_vocabulary.is_bool(ty))
        .count();
    if bools > cfg.max_bool_params {
        out.push(make(
            SmellKind::BooleanBlindness,
            format!(
                "{bools} boolean parameters — call sites are unreadable; consider {}, or split the function",
                ctx.wording.options
            ),
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
    m: &FunctionUnit,
    cfg: &Smells,
    out: &mut Vec<SmellFinding>,
) {
    let annotated = ctx
        .type_hints
        .return_type(m)
        .filter(|ty| matches!(base_type(ty), "tuple" | "Tuple"))
        .map(|ty| tuple_arity(ty))
        .unwrap_or(0);
    let arity = m.max_tuple_return.max(annotated);
    let vector_like =
        cfg.tuple_allow_homogeneous && m.homogeneous_tuple_return && arity <= MAX_HOMOGENEOUS_TUPLE;
    if arity > cfg.max_tuple_return && !vector_like {
        out.push(make(
            SmellKind::TuplePacking,
            format!(
                "returns a {arity}-element tuple — positional grouped data; consider {}",
                ctx.wording.grouped_value
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
    // Returning a top type hands the narrowing burden to every caller.
    let kind = match ctx.type_vocabulary.loose_kind(ty)? {
        LooseTypeKind::SafeTop => LooseTypeKind::SchemaErasing,
        kind => kind,
    };
    reportable(kind, strictness).map(|kind| (ty, kind))
}

fn reportable_loose(
    ctx: &SmellContext<'_>,
    annotation: &str,
    strictness: Strictness,
) -> Option<LooseTypeKind> {
    reportable(ctx.type_vocabulary.loose_kind(annotation)?, strictness)
}

fn reportable(kind: LooseTypeKind, strictness: Strictness) -> Option<LooseTypeKind> {
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
        if !loose && !ctx.type_vocabulary.is_primitive_scalar_alias(&alias.target) {
            continue;
        }
        out.push(make(
            SmellKind::LooseTyping,
            format!(
                "type alias {} = {} hides a loose type-safety contract — replace it with {} instead of a shallow alias",
                alias.name,
                alias.target,
                ctx.wording.structure
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
    split_top_level(body, ',').len()
}
