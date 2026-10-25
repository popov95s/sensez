//! `implicit_schema`: one receiver read with many distinct string keys.

use super::{make, SmellContext};
use crate::config::smells::Smells;
use crate::profiles::typevocab::base_type;
use crate::report::{Severity, SmellFinding, SmellKind};
use crate::spine::ir::FunctionUnit;
use std::collections::{HashMap, HashSet};

/// A receiver subscripted with ≥ N distinct string-literal keys — an implicit
/// schema that wants a dataclass. Receivers annotated as a non-dict type
/// (DataFrame, ndarray, ...) are skipped; dict-annotated or unknown both flag.
pub(super) fn detect(
    ctx: &SmellContext<'_>,
    m: &FunctionUnit,
    schemas: &HashMap<String, HashSet<String>>,
    cfg: &Smells,
    out: &mut Vec<SmellFinding>,
) {
    for (recv, keys) in &with_destructured_keys(ctx, m, schemas) {
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

/// Destructured keys join a receiver's schema only when it is annotated as a loose map.
fn with_destructured_keys(
    ctx: &SmellContext<'_>,
    m: &FunctionUnit,
    schemas: &HashMap<String, HashSet<String>>,
) -> HashMap<String, HashSet<String>> {
    let mut merged = schemas.clone();
    for (recv, keys) in &m.destructured_keys {
        let loose = ctx.type_hints.param_type(m, recv).is_some_and(|ty| {
            !ty.trim_start().starts_with('{') && ctx.type_vocabulary.loose_kind(ty).is_some()
        });
        if loose {
            merged
                .entry(recv.clone())
                .or_default()
                .extend(keys.iter().cloned());
        }
    }
    merged
}

pub(super) fn propagated_schemas(
    metrics: &[FunctionUnit],
) -> Vec<HashMap<String, HashSet<String>>> {
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
