//! Performance-oriented local smells.

mod external;

use super::{make, SmellContext};
use crate::config::smells::Smells;
use crate::profiles::{registry, PerformanceProfile};
use crate::report::{Severity, SmellFinding, SmellKind};
use crate::spine::ir::{CallFact, FunctionUnit, IterationPass};
use external::external_calls;
use std::collections::BTreeMap;

pub fn detect(ctx: &SmellContext<'_>, metrics: &[FunctionUnit], cfg: &Smells) -> Vec<SmellFinding> {
    // Performance smells need to look up callees by name to attribute
    // helper-in-loop work to the caller; same-named units stay ambiguous.
    let mut functions: BTreeMap<&str, Vec<&FunctionUnit>> = BTreeMap::new();
    for m in metrics {
        functions.entry(m.name.as_str()).or_default().push(m);
    }
    let profile = registry::performance_profile(ctx.language);
    metrics
        .iter()
        .flat_map(|m| {
            let mut findings = direct_findings(ctx, m, cfg, &functions, profile);
            findings.extend(helper_findings(ctx, m, &functions, profile));
            findings
        })
        .collect()
}

fn direct_findings(
    ctx: &SmellContext<'_>,
    m: &FunctionUnit,
    cfg: &Smells,
    functions: &BTreeMap<&str, Vec<&FunctionUnit>>,
    profile: &dyn PerformanceProfile,
) -> Vec<SmellFinding> {
    let mut out = Vec::new();
    let nested_loops = significant_loops(profile, &m.performance.nested_loops);
    if let Some(first) = nested_loops.first() {
        out.push(finding(
            SmellKind::NestedLoop,
            ctx,
            m,
            first.line,
            nested_loops.len(),
            "nested loop multiplies work per input item",
            Severity::Warning,
        ));
    }
    if let Some(first) = m.performance.sorts_in_loops.first() {
        out.push(finding(
            SmellKind::SortInLoop,
            ctx,
            m,
            first.line,
            m.performance.sorts_in_loops.len(),
            "sort inside a loop repeats O(n log n) work",
            Severity::Warning,
        ));
    }
    let in_tests = ctx.test_source || m.review_risks.is_test_case;
    if cfg.repeated_iteration_include_tests || !in_tests {
        for calls in repeated_iterations(m, cfg, profile) {
            out.push(finding(
                SmellKind::RepeatedIteration,
                ctx,
                m,
                calls[0].line,
                calls.len(),
                &repeated_iteration_reason(&calls),
                Severity::Warning,
            ));
        }
    }
    for call in external_calls(ctx, m, &m.performance.loop_calls, functions, profile).values() {
        out.push(finding(
            SmellKind::NPlusOneCall,
            ctx,
            m,
            call.line,
            1,
            "external-looking call runs once per loop iteration",
            Severity::Info,
        ));
    }
    out
}

fn helper_findings(
    ctx: &SmellContext<'_>,
    m: &FunctionUnit,
    functions: &BTreeMap<&str, Vec<&FunctionUnit>>,
    profile: &dyn PerformanceProfile,
) -> Vec<SmellFinding> {
    let mut out = Vec::new();
    for call in &m.performance.loop_calls {
        if call.target == m.name {
            continue;
        }
        let Some(&[callee]) = functions.get(call.target.as_str()).map(Vec::as_slice) else {
            continue;
        };
        let callee_loops = significant_loops(profile, &callee.performance.loops);
        if !callee_loops.is_empty() {
            out.push(finding(
                SmellKind::NestedLoop,
                ctx,
                m,
                call.line,
                callee_loops.len() + 1,
                "helper called in a loop also iterates",
                Severity::Warning,
            ));
        }
        if !external_calls(ctx, callee, &callee.performance.calls, functions, profile).is_empty() {
            out.push(finding(
                SmellKind::NPlusOneCall,
                ctx,
                m,
                call.line,
                1,
                "helper called in a loop performs external-looking calls",
                Severity::Info,
            ));
        }
    }
    out
}

fn repeated_iterations<'a>(
    m: &'a FunctionUnit,
    cfg: &Smells,
    profile: &dyn PerformanceProfile,
) -> Vec<Vec<&'a CallFact>> {
    let mut by_base: BTreeMap<(&str, usize), Vec<&CallFact>> = BTreeMap::new();
    let counted = m.performance.iteration_calls.iter().filter(|call| {
        !call.base.is_empty()
            && call.pass != IterationPass::ShortCircuit
            && (cfg.repeated_iteration_include_tests || !call.in_assertion)
    });
    for call in counted {
        by_base
            .entry((call.base.as_str(), call.region))
            .or_default()
            .push(call);
    }
    by_base
        .into_iter()
        .filter(|(_, calls)| {
            calls
                .iter()
                .any(|call| call.pass != IterationPass::Projection)
        })
        .filter(|((base, _), calls)| has_fusable_pair(m, profile, base, calls))
        .map(|(_, calls)| calls)
        .collect()
}

fn has_fusable_pair(
    m: &FunctionUnit,
    profile: &dyn PerformanceProfile,
    base: &str,
    calls: &[&CallFact],
) -> bool {
    let mut ordered = calls.to_vec();
    ordered.sort_by_key(|call| (call.line, call.offset));
    ordered.iter().enumerate().any(|(index, first)| {
        ordered[index + 1..].iter().any(|second| {
            !mutation_between(profile, &m.performance.calls, base, first, second)
                && !exit_between(m, first, second)
                && !depends_on_binding_since(m, first, second)
        })
    })
}

/// True when `base` is mutated on any line from the first pass through the second.
fn mutation_between(
    profile: &dyn PerformanceProfile,
    all_calls: &[CallFact],
    base: &str,
    first: &CallFact,
    second: &CallFact,
) -> bool {
    all_calls.iter().any(|call| {
        call.base == base
            && (first.offset..=second.offset).contains(&call.offset)
            && profile.is_mutating_call(&call.method)
    })
}

fn exit_between(m: &FunctionUnit, first: &CallFact, second: &CallFact) -> bool {
    m.performance
        .exits
        .iter()
        .any(|&offset| first.offset < offset && offset < second.offset)
}

fn depends_on_binding_since(m: &FunctionUnit, first: &CallFact, second: &CallFact) -> bool {
    m.performance.bindings.iter().any(|(name, end)| {
        first.offset < *end && *end <= second.offset && second.arg_idents.contains(name)
    })
}

fn repeated_iteration_reason(calls: &[&CallFact]) -> String {
    let lines = super::lines::hit_lines(calls.iter().map(|call| call.line));
    let mut methods: Vec<&str> = calls.iter().map(|call| call.method.as_str()).collect();
    methods.sort_unstable();
    methods.dedup();
    let lines = lines
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "`{}` is iterated {} times in this scope (lines {lines}: {})",
        calls[0].base,
        calls.len(),
        methods.join(", ")
    )
}

fn significant_loops<'a>(
    profile: &dyn PerformanceProfile,
    loops: &'a [crate::spine::ir::PerfLine],
) -> Vec<&'a crate::spine::ir::PerfLine> {
    loops
        .iter()
        .filter(|line| !profile.is_bounded_loop(&line.subject))
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn finding(
    kind: SmellKind,
    ctx: &SmellContext<'_>,
    m: &FunctionUnit,
    line: usize,
    metric: usize,
    reason: &str,
    severity: Severity,
) -> SmellFinding {
    make(
        kind,
        format!("{reason}; combine the work or use a bulk operation."),
        ctx.path,
        line,
        &m.name,
        severity,
        metric as u32,
        1,
    )
}
