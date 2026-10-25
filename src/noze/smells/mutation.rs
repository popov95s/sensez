//! Mutation & stringly-typed discipline smells.
//!
//! - `mutated_parameter`: caller-visible mutation of an input parameter.
//! - `reassigned_parameter` (opt-in): rebinding a parameter name.
//! - `implicit_schema`: one receiver subscripted with many distinct string keys.
//! - `literal_membership`: branching on `x in ["a", "b"]` string lists.

use super::external_vocab::is_external_vocabulary;
use super::lines::{hit_lines, line_list};
use super::{make, SmellContext};
use crate::config::smells::Smells;
use crate::profiles::typevocab::base_type;
use crate::report::{Severity, SmellFinding, SmellKind};
use crate::spine::ir::{
    CallbackPosition, FunctionUnit, LiteralMembership, MutatorFamily, Visibility,
};
use std::collections::{HashMap, HashSet};

pub fn detect(
    ctx: &SmellContext<'_>,
    metrics: &[FunctionUnit],
    cfg: &Smells,
    out: &mut Vec<SmellFinding>,
) {
    let schemas = propagated_schemas(metrics);
    for (index, m) in metrics.iter().enumerate() {
        if cfg.is_enabled(SmellKind::MutatedParameter) {
            mutated_params(ctx, m, cfg, out);
        }
        if cfg.is_enabled(SmellKind::ReassignedParameter) {
            reassigned_params(ctx, m, out);
        }
        if cfg.implicit_schema_min_keys > 0 {
            implicit_schema(ctx, m, &schemas[index], cfg, out);
        }
        if cfg.is_enabled(SmellKind::LiteralMembership) {
            literal_membership(ctx, m, cfg, out);
        }
    }
}

fn literal_membership(
    ctx: &SmellContext<'_>,
    m: &FunctionUnit,
    cfg: &Smells,
    out: &mut Vec<SmellFinding>,
) {
    let hits: Vec<&LiteralMembership> = m
        .literal_memberships
        .iter()
        .filter(|test| {
            !(cfg.literal_membership_skip_external_vocab && is_external_vocabulary(&test.literals))
        })
        .filter(|test| {
            test.operand
                .as_deref()
                .is_none_or(|operand| !has_literal_union_type(ctx, m, operand))
        })
        .collect();
    let lines = hit_lines(hits.iter().map(|test| test.line));
    let Some(&first) = lines.first() else {
        return;
    };
    let finding = make(
        SmellKind::LiteralMembership,
        format!(
            "{} membership test(s) against literal string lists at {} — stringly-typed categories; consider {}",
            hits.len(),
            line_list(&lines),
            ctx.wording.category
        ),
        ctx.path,
        first,
        &m.name,
        Severity::Info,
        hits.len() as u32,
        0,
    );
    out.push(SmellFinding { lines, ..finding });
}

/// The operand is already annotated with a closed literal type (inline or a same-file alias).
fn has_literal_union_type(ctx: &SmellContext<'_>, m: &FunctionUnit, operand: &str) -> bool {
    let Some(annotation) = ctx
        .type_hints
        .param_type(m, operand)
        .or_else(|| ctx.type_hints.var_types.get(operand))
    else {
        return false;
    };
    let vocab = ctx.type_vocabulary;
    vocab.is_literal_union(annotation)
        || ctx
            .type_hints
            .type_aliases
            .iter()
            .any(|alias| alias.name == annotation.trim() && vocab.is_literal_union(&alias.target))
}

/// Parameters whose *object* is mutated in the body (subscript-assign, `del`,
/// or a mutating method call) — a caller-visible side effect. With
/// `param_attr_mutation`, also counts mutation reached through a parameter's
/// attribute (`p.kwargs[k]=v`); `self`/`cls` stay excluded either way.
fn mutated_params(
    ctx: &SmellContext<'_>,
    m: &FunctionUnit,
    cfg: &Smells,
    out: &mut Vec<SmellFinding>,
) {
    // An inline callback's params are elements of the caller's own data.
    if m.callback_position == CallbackPosition::Argument {
        return;
    }
    let mutated: Vec<&str> = m
        .param_names
        .iter()
        .chain(&m.destructured_params)
        .filter(|p| !matches!(p.as_str(), "self" | "cls"))
        .filter(|p| {
            m.mutated_names.contains(*p)
                || is_method_mutated(ctx, m, p)
                || (cfg.param_attr_mutation && m.attr_mutated_names.contains(*p))
        })
        .map(String::as_str)
        .collect();
    if mutated.is_empty() || is_private_collector(ctx, m, &mutated) {
        return;
    }
    let (severity, advice) = mutation_advice(m, &mutated);
    let lines = hit_lines(
        m.mutation_lines
            .iter()
            .filter(|(name, _)| mutated.contains(&name.as_str()))
            .map(|(_, line)| *line),
    );
    let finding = make(
        SmellKind::MutatedParameter,
        format!(
            "mutates input parameter(s) [{}] — {advice}",
            mutated.join(", ")
        ),
        ctx.path,
        lines.first().copied().unwrap_or(m.start_line),
        &m.name,
        severity,
        mutated.len() as u32,
        0,
    );
    out.push(SmellFinding { lines, ..finding });
}

/// A mutating method call counts only when the param's declared type (if any)
/// is a built-in collection that method family mutates.
fn is_method_mutated(ctx: &SmellContext<'_>, m: &FunctionUnit, param: &str) -> bool {
    let declared = ctx.type_hints.param_type(m, param);
    m.method_mutations.iter().any(|(name, family)| {
        name == param
            && declared.is_none_or(|ty| ctx.type_vocabulary.is_mutable_collection(ty, *family))
    })
}

fn mutation_advice(m: &FunctionUnit, mutated: &[&str]) -> (Severity, &'static str) {
    if mutated.iter().any(|p| m.returned_names.contains(*p)) {
        return (
            Severity::Warning,
            "mutates and returns its input; callers can't tell whether it copies",
        );
    }
    (
        Severity::Warning,
        "caller-visible side effect; return a new value instead",
    )
}

/// A private void helper filling its last parameter, or parameters typed as collections.
fn is_private_collector(ctx: &SmellContext<'_>, m: &FunctionUnit, mutated: &[&str]) -> bool {
    let returns_nothing = ctx
        .type_hints
        .return_type(m)
        .is_none_or(|ty| ctx.type_vocabulary.is_void(ty));
    let only_last = m
        .param_names
        .last()
        .is_some_and(|last| mutated == [last.as_str()]);
    let typed_collections = mutated.iter().all(|p| is_typed_collection(ctx, m, p));
    m.visibility == Visibility::Private
        && returns_nothing
        && !mutated.iter().any(|p| m.returned_names.contains(*p))
        && (only_last || typed_collections)
}

fn is_typed_collection(ctx: &SmellContext<'_>, m: &FunctionUnit, param: &str) -> bool {
    let vocab = ctx.type_vocabulary;
    ctx.type_hints.param_type(m, param).is_some_and(|ty| {
        vocab.is_mutable_collection(ty, MutatorFamily::Array)
            || vocab.is_mutable_collection(ty, MutatorFamily::Collection)
    })
}

/// Parameters rebound with plain assignment (opt-in: `x = x or []` is idiomatic).
fn reassigned_params(ctx: &SmellContext<'_>, m: &FunctionUnit, out: &mut Vec<SmellFinding>) {
    let rebound: Vec<&str> = m
        .param_names
        .iter()
        .filter(|p| !matches!(p.as_str(), "self" | "cls"))
        .filter(|p| m.local_reassigns.contains_key(*p))
        .map(String::as_str)
        .collect();
    if rebound.is_empty() {
        return;
    }
    out.push(make(
        SmellKind::ReassignedParameter,
        format!("rebinds input parameter(s) [{}]", rebound.join(", ")),
        ctx.path,
        m.start_line,
        &m.name,
        Severity::Info,
        rebound.len() as u32,
        0,
    ));
}

/// A receiver subscripted with ≥ N distinct string-literal keys — an implicit
/// schema that wants a dataclass. Receivers annotated as a non-dict type
/// (DataFrame, ndarray, ...) are skipped; dict-annotated or unknown both flag.
fn implicit_schema(
    ctx: &SmellContext<'_>,
    m: &FunctionUnit,
    schemas: &HashMap<String, HashSet<String>>,
    cfg: &Smells,
    out: &mut Vec<SmellFinding>,
) {
    for (recv, keys) in schemas {
        if keys.len() < cfg.implicit_schema_min_keys
            || m.dynamic_key_receivers.contains(recv)
            || keys.iter().all(|key| key.starts_with('-'))
        {
            continue;
        }
        let annotated = ctx
            .type_hints
            .param_type(m, recv)
            .or_else(|| ctx.type_hints.var_types.get(recv));
        if annotated.is_some_and(|ty| {
            !ctx.type_vocabulary.is_dictish(ty) || ctx.type_vocabulary.is_open_map(ty)
        }) {
            continue;
        }
        if m.validated_names.contains(recv) || is_validated_boundary(ctx, m) {
            continue;
        }
        out.push(make(
            SmellKind::ImplicitSchema,
            format!(
                "`{recv}` accessed via {} distinct string keys — implicit schema; consider {}",
                keys.len(),
                ctx.wording.structure
            ),
            ctx.path,
            m.str_key_lines.get(recv).copied().unwrap_or(m.start_line),
            &m.name,
            Severity::Info,
            keys.len() as u32,
            cfg.implicit_schema_min_keys as u32,
        ));
    }
}

fn propagated_schemas(metrics: &[FunctionUnit]) -> Vec<HashMap<String, HashSet<String>>> {
    let by_name: HashMap<&str, usize> = metrics
        .iter()
        .enumerate()
        .map(|(index, metric)| (metric.name.as_str(), index))
        .collect();
    let mut schemas: Vec<_> = metrics
        .iter()
        .map(|metric| metric.str_keys.clone())
        .collect();
    for _ in 0..metrics.len().min(4) {
        let snapshot = schemas.clone();
        for (caller_index, caller) in metrics.iter().enumerate() {
            for call in &caller.schema_calls {
                let Some(&callee_index) = by_name.get(call.target.as_str()) else {
                    continue;
                };
                for (position, argument) in call.arguments.iter().enumerate() {
                    if !caller.param_names.contains(argument) {
                        continue;
                    }
                    let Some(parameter) = metrics[callee_index].param_names.get(position) else {
                        continue;
                    };
                    let Some(keys) = snapshot[callee_index].get(parameter) else {
                        continue;
                    };
                    schemas[caller_index]
                        .entry(argument.clone())
                        .or_default()
                        .extend(keys.iter().cloned());
                }
            }
        }
    }
    schemas
}

fn is_validated_boundary(ctx: &SmellContext<'_>, metric: &FunctionUnit) -> bool {
    ctx.type_hints
        .return_type(metric)
        .is_some_and(|return_type| {
            ctx.type_vocabulary.has_domain_model(return_type)
                && metric
                    .returned_constructors
                    .contains(base_type(return_type))
        })
}
