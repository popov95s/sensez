//! Performance-oriented local smells.

mod external;

use super::{make, SmellContext};
use crate::config::smells::Smells;
use crate::profiles::{registry, PerformanceProfile};
use crate::report::{Severity, SmellFinding, SmellKind};
use crate::spine::ir::{CallFact, FunctionUnit};
use external::external_calls;
use std::collections::BTreeMap;

pub fn detect(
    ctx: &SmellContext<'_>,
    metrics: &[FunctionUnit],
    _cfg: &Smells,
) -> Vec<SmellFinding> {
    // Performance smells need to look up callees by name to attribute
    // helper-in-loop work to the caller, so we keep a per-name view of the
    // metrics by name. The map is name → *metrics*, not name → *unit*, but
    // the only field read for the lookup is `performance`.
    let functions: BTreeMap<&str, &FunctionUnit> =
        metrics.iter().map(|m| (m.name.as_str(), m)).collect();
    let profile = registry::performance_profile(ctx.language);
    metrics
        .iter()
        .flat_map(|m| {
            let mut findings = direct_findings(ctx, m, &functions, profile);
            findings.extend(helper_findings(ctx, m, &functions, profile));
            findings
        })
        .collect()
}

fn direct_findings(
    ctx: &SmellContext<'_>,
    m: &FunctionUnit,
    functions: &BTreeMap<&str, &FunctionUnit>,
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
    for calls in repeated_iterations(m, profile).values() {
        out.push(finding(
            SmellKind::RepeatedIteration,
            ctx,
            m,
            calls[0].line,
            calls.len(),
            "same collection is iterated multiple times in this scope",
            Severity::Warning,
        ));
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
    functions: &BTreeMap<&str, &FunctionUnit>,
    profile: &dyn PerformanceProfile,
) -> Vec<SmellFinding> {
    let mut out = Vec::new();
    for call in &m.performance.loop_calls {
        if call.target == m.name {
            continue;
        }
        let Some(callee) = functions.get(call.target.as_str()).copied() else {
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
    profile: &dyn PerformanceProfile,
) -> BTreeMap<(&'a str, usize), Vec<&'a CallFact>> {
    let mut by_base: BTreeMap<(&str, usize), Vec<&CallFact>> = BTreeMap::new();
    for call in &m.performance.iteration_calls {
        if !call.base.is_empty() {
            by_base
                .entry((call.base.as_str(), call.region))
                .or_default()
                .push(call);
        }
    }
    by_base.retain(|(base, _), calls| {
        calls.len() > 1 && !mutation_between(profile, &m.performance.calls, base, calls)
    });
    by_base
}

fn mutation_between(
    profile: &dyn PerformanceProfile,
    all_calls: &[CallFact],
    base: &str,
    iterations: &[&CallFact],
) -> bool {
    let first = iterations.iter().map(|call| call.line).min().unwrap_or(0);
    let last = iterations.iter().map(|call| call.line).max().unwrap_or(0);
    all_calls.iter().any(|call| {
        call.base == base
            && first < call.line
            && call.line < last
            && profile.is_mutating_call(&call.method)
    })
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
