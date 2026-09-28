//! Structurally derived risks that warrant focused review.

use super::{make, SmellContext};
use crate::config::smells::Smells;
use crate::report::{Severity, SmellFinding, SmellKind};
use crate::spine::ir::{ClassUnit, FunctionUnit};
use std::collections::{HashMap, HashSet};

pub fn detect(
    ctx: &SmellContext<'_>,
    metrics: &[FunctionUnit],
    classes: &[ClassUnit],
    config: &Smells,
    out: &mut Vec<SmellFinding>,
) {
    for metric in metrics {
        defensive_fallback(ctx, metric, out);
        redundant_validation(ctx, metric, out);
    }
    wrapper_chains(ctx, metrics, config.max_wrapper_depth, out);
    divergent_abstractions(ctx, classes, out);
}

fn wrapper_chains(
    ctx: &SmellContext<'_>,
    metrics: &[FunctionUnit],
    max_depth: usize,
    out: &mut Vec<SmellFinding>,
) {
    let wrappers: Vec<_> = metrics
        .iter()
        .filter(|unit| {
            !unit.is_method && !unit.is_nested && unit.review_risks.forwards_to.is_some()
        })
        .collect();
    let forwarded_to: HashSet<_> = wrappers
        .iter()
        .filter_map(|unit| unit.review_risks.forwards_to.as_deref())
        .collect();

    let mut by_name = HashMap::new();
    let mut ambiguous = HashSet::new();
    for unit in metrics.iter().filter(|unit| !unit.is_method && !unit.is_nested) {
        if by_name.insert(unit.name.as_str(), unit).is_some() {
            ambiguous.insert(unit.name.as_str());
        }
    }

    // Begin at chain roots so each local forwarding chain produces one finding.
    for outer in wrappers {
        if forwarded_to.contains(outer.name.as_str()) || ambiguous.contains(outer.name.as_str()) {
            continue;
        }
        let mut chain = Vec::new();
        let mut visited = HashSet::new();
        let mut current = outer;
        let terminal = loop {
            if !visited.insert(current.name.as_str()) || ambiguous.contains(current.name.as_str()) {
                break None;
            }
            let Some(target) = current.review_risks.forwards_to.as_deref() else {
                break Some(current);
            };
            chain.push(current);
            let Some(next) = by_name.get(target).copied() else {
                break None;
            };
            current = next;
        };
        if terminal.is_none() || chain.len() <= max_depth {
            continue;
        }
        out.push(make(
            SmellKind::RedundantWrapperChain,
            format!(
                "{} is part of a {}-layer unchanged forwarding chain ending at {}; remove pass-through functions or configure max_depth",
                outer.name,
                chain.len(),
                terminal.unwrap().name
            ),
            ctx.path,
            outer.start_line,
            &outer.name,
            Severity::Info,
            chain.len() as u32,
            max_depth as u32,
        ));
    }
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
