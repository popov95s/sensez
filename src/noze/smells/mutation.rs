//! Mutation & stringly-typed discipline smells.
//!
//! - `mutated_parameter`: caller-visible mutation of an input parameter.
//! - `reassigned_parameter` (opt-in): rebinding a parameter name.
//! - `implicit_schema`: one receiver subscripted with many distinct string keys.
//! - `literal_membership`: branching on `x in ["a", "b"]` string lists.

use super::{make, structure_target, SmellContext};
use crate::config::smells::Smells;
use crate::profiles::typevocab::base_type;
use crate::report::{Severity, SmellFinding, SmellKind};
use crate::spine::ir::FunctionMetrics;
use std::collections::{HashMap, HashSet};

pub fn detect(
    ctx: &SmellContext<'_>,
    metrics: &[FunctionMetrics],
    cfg: &Smells,
    out: &mut Vec<SmellFinding>,
) {
    let schemas = propagated_schemas(metrics);
    for (index, m) in metrics.iter().enumerate() {
        if cfg.param_mutation {
            mutated_params(ctx, m, cfg, out);
        }
        if cfg.param_reassignment {
            reassigned_params(ctx, m, out);
        }
        if cfg.implicit_schema_min_keys > 0 {
            implicit_schema(ctx, m, &schemas[index], cfg, out);
        }
        if cfg.literal_membership && m.literal_membership_tests > 0 {
            out.push(make(
                SmellKind::LiteralMembership,
                format!(
                    "{} membership test(s) against literal string lists — stringly-typed categories; consider an Enum",
                    m.literal_membership_tests
                ),
                ctx.path,
                m.start_line,
                &m.name,
                Severity::Info,
                m.literal_membership_tests as u32,
                0,
            ));
        }
    }
}

/// Parameters whose *object* is mutated in the body (subscript-assign, `del`,
/// or a mutating method call) — a caller-visible side effect. With
/// `param_attr_mutation`, also counts mutation reached through a parameter's
/// attribute (`p.kwargs[k]=v`); `self`/`cls` stay excluded either way.
fn mutated_params(
    ctx: &SmellContext<'_>,
    m: &FunctionMetrics,
    cfg: &Smells,
    out: &mut Vec<SmellFinding>,
) {
    let mutated: Vec<&str> = m
        .param_names
        .iter()
        .filter(|p| !matches!(p.as_str(), "self" | "cls"))
        .filter(|p| {
            m.mutated_names.contains(*p)
                || (cfg.param_attr_mutation && m.attr_mutated_names.contains(*p))
        })
        .map(String::as_str)
        .collect();
    if mutated.is_empty() {
        return;
    }
    out.push(make(
        SmellKind::MutatedParameter,
        format!(
            "mutates input parameter(s) [{}] — caller-visible side effect; return a new value instead",
            mutated.join(", ")
        ),
        ctx.path,
        m.start_line,
        &m.name,
        Severity::Warning,
        mutated.len() as u32,
        0,
    ));
}

/// Parameters rebound with plain assignment (opt-in: `x = x or []` is idiomatic).
fn reassigned_params(ctx: &SmellContext<'_>, m: &FunctionMetrics, out: &mut Vec<SmellFinding>) {
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
    m: &FunctionMetrics,
    schemas: &HashMap<String, HashSet<String>>,
    cfg: &Smells,
    out: &mut Vec<SmellFinding>,
) {
    for (recv, keys) in schemas {
        if keys.len() < cfg.implicit_schema_min_keys {
            continue;
        }
        let annotated = ctx
            .type_hints
            .param_types
            .get(&(m.name.clone(), recv.clone()))
            .or_else(|| ctx.type_hints.var_types.get(recv));
        if annotated.is_some_and(|ty| !ctx.type_vocabulary.is_dictish(ty)) {
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
                structure_target(ctx.language)
            ),
            ctx.path,
            m.start_line,
            &m.name,
            Severity::Info,
            keys.len() as u32,
            cfg.implicit_schema_min_keys as u32,
        ));
    }
}

fn propagated_schemas(metrics: &[FunctionMetrics]) -> Vec<HashMap<String, HashSet<String>>> {
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

fn is_validated_boundary(ctx: &SmellContext<'_>, metric: &FunctionMetrics) -> bool {
    ctx.type_hints
        .return_types
        .get(&metric.name)
        .is_some_and(|return_type| {
            ctx.type_vocabulary.has_domain_model(return_type)
                && metric
                    .returned_constructors
                    .contains(base_type(return_type))
        })
}
