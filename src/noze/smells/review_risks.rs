//! Structurally derived risks that warrant focused review.

use super::{make, SmellContext};
use crate::report::{Severity, SmellFinding, SmellKind};
use crate::spine::ir::{ClassUnit, FunctionUnit};
use std::collections::HashSet;

pub fn detect(
    ctx: &SmellContext<'_>,
    metrics: &[FunctionUnit],
    classes: &[ClassUnit],
    out: &mut Vec<SmellFinding>,
) {
    for metric in metrics {
        defensive_fallback(ctx, metric, out);
        redundant_validation(ctx, metric, out);
    }
    divergent_abstractions(ctx, classes, out);
}

fn defensive_fallback(ctx: &SmellContext<'_>, metric: &FunctionUnit, out: &mut Vec<SmellFinding>) {
    let facts = &metric.review_risks;
    if facts.broad_handlers == 0 || facts.empty_fallbacks < 2 {
        return;
    }
    out.push(make(
        SmellKind::DefensiveFallback,
        "broad error handling combines with repeated empty fallbacks, hiding contract failures"
            .to_string(),
        ctx.path,
        metric.start_line,
        &metric.name,
        Severity::Warning,
        (facts.broad_handlers + facts.empty_fallbacks) as u32,
        3,
    ));
}

fn redundant_validation(
    ctx: &SmellContext<'_>,
    metric: &FunctionUnit,
    out: &mut Vec<SmellFinding>,
) {
    let reassigned_local = metric.local_reassigns.values().any(|count| *count > 1);
    if metric.review_risks.repeated_guards == 0 || reassigned_local {
        return;
    }
    out.push(make(
        SmellKind::RedundantValidation,
        "the same guard is checked repeatedly on one function path".to_string(),
        ctx.path,
        metric.start_line,
        &metric.name,
        Severity::Info,
        metric.review_risks.repeated_guards as u32,
        0,
    ));
}

fn divergent_abstractions(
    ctx: &SmellContext<'_>,
    classes: &[ClassUnit],
    out: &mut Vec<SmellFinding>,
) {
    for abstraction in classes.iter().filter(|class| class.is_abstract) {
        let implementations: Vec<_> = classes
            .iter()
            .filter(|class| {
                !class.is_abstract
                    && class
                        .bases
                        .iter()
                        .any(|base| base.rsplit('.').next() == Some(abstraction.name.as_str()))
            })
            .collect();
        if implementations.len() != 2 || !implementations_diverge(abstraction, &implementations) {
            continue;
        }
        out.push(make(
            SmellKind::DivergentAbstraction,
            format!(
                "{} has only two implementations with substantially different responsibilities",
                abstraction.name
            ),
            ctx.path,
            abstraction.start_line,
            &abstraction.name,
            Severity::Warning,
            2,
            2,
        ));
    }
}

fn implementations_diverge(abstraction: &ClassUnit, implementations: &[&ClassUnit]) -> bool {
    let contract: HashSet<_> = abstraction.methods.iter().map(String::as_str).collect();
    let left = specific_methods(implementations[0], &contract);
    let right = specific_methods(implementations[1], &contract);
    if left.len() < 2 || right.len() < 2 {
        return false;
    }
    let overlap = left.intersection(&right).count();
    let union = left.union(&right).count();
    union > 0 && overlap * 3 < union
}

fn specific_methods<'a>(class: &'a ClassUnit, contract: &HashSet<&str>) -> HashSet<&'a str> {
    class
        .methods
        .iter()
        .map(String::as_str)
        .filter(|name| !contract.contains(name))
        .collect()
}
