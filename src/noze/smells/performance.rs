//! Performance-oriented local smells.

use super::{make, SmellContext};
use crate::config::smells::Smells;
use crate::noze::{Severity, SmellFinding, SmellKind};
use crate::profiles::{registry, PerformanceProfile};
use crate::spine::ir::{CallFact, FunctionMetrics, PerfLine};
use std::collections::BTreeMap;

pub fn detect(
    ctx: &SmellContext<'_>,
    metrics: &[FunctionMetrics],
    _cfg: &Smells,
    out: &mut Vec<SmellFinding>,
) {
    // Performance smells need to look up callees by name to attribute
    // helper-in-loop work to the caller, so we keep a per-name view of the
    // metrics by name. The map is name → *metrics*, not name → *unit*, but
    // the only field read for the lookup is `performance`.
    let functions: BTreeMap<&str, &FunctionMetrics> =
        metrics.iter().map(|m| (m.name.as_str(), m)).collect();
    let profile = registry::performance_profile(ctx.language);
    for m in metrics {
        direct_findings(ctx, m, &functions, profile, out);
        helper_findings(ctx, m, &functions, profile, out);
    }
}

fn direct_findings(
    ctx: &SmellContext<'_>,
    m: &FunctionMetrics,
    functions: &BTreeMap<&str, &FunctionMetrics>,
    profile: &dyn PerformanceProfile,
    out: &mut Vec<SmellFinding>,
) {
    let nested_loops = significant_loops(&m.performance.nested_loops);
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
    for calls in repeated_iterations(m).values() {
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
    for call in external_calls(&m.performance.loop_calls, functions, profile).values() {
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
}

fn helper_findings(
    ctx: &SmellContext<'_>,
    m: &FunctionMetrics,
    functions: &BTreeMap<&str, &FunctionMetrics>,
    profile: &dyn PerformanceProfile,
    out: &mut Vec<SmellFinding>,
) {
    for call in &m.performance.loop_calls {
        let Some(callee) = functions.get(call.target.as_str()).copied() else {
            continue;
        };
        let callee_loops = significant_loops(&callee.performance.loops);
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
        if !external_calls(&callee.performance.calls, functions, profile).is_empty() {
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
}

fn repeated_iterations(m: &FunctionMetrics) -> BTreeMap<&str, Vec<&CallFact>> {
    let mut by_base: BTreeMap<&str, Vec<&CallFact>> = BTreeMap::new();
    for call in &m.performance.iteration_calls {
        if !call.base.is_empty() {
            by_base.entry(call.base.as_str()).or_default().push(call);
        }
    }
    by_base.retain(|_, calls| calls.len() > 1);
    by_base
}

fn external_calls<'a>(
    calls: &'a [CallFact],
    functions: &BTreeMap<&str, &FunctionMetrics>,
    profile: &dyn PerformanceProfile,
) -> BTreeMap<&'a str, &'a CallFact> {
    let mut out = BTreeMap::new();
    for call in calls {
        if functions.contains_key(call.target.as_str()) {
            continue;
        }
        if is_external(call, profile) {
            out.entry(call.target.as_str()).or_insert(call);
        }
    }
    out
}

fn significant_loops(loops: &[PerfLine]) -> Vec<&PerfLine> {
    loops
        .iter()
        .filter(|line| !is_bounded_constant(&line.subject))
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn finding(
    kind: SmellKind,
    ctx: &SmellContext<'_>,
    m: &FunctionMetrics,
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

fn is_external(call: &CallFact, profile: &dyn PerformanceProfile) -> bool {
    call.member
        && (profile.is_expensive_loop_call(&call.method)
            || (call.method == "get" && profile.is_external_get_receiver(&call.base)))
}

fn is_bounded_constant(subject: &str) -> bool {
    !subject.is_empty()
        && subject
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch == '_')
}
