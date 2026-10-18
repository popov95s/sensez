//! Performance-oriented local smells.

mod external;
mod loop_context;

use super::{make, SmellContext};
use crate::config::smells::Smells;
use crate::profiles::{registry, PerformanceProfile};
use crate::report::{Severity, SmellFinding, SmellKind};
use crate::spine::ir::{CallFact, FunctionUnit, IterationPass, PerfLine};
use external::{external_calls, LoopCallRules};
use loop_context::{nested_loop_reason, owned_by_loop, repeats_per_item, walks_outer_item};
use std::collections::BTreeMap;

pub fn detect(ctx: &SmellContext<'_>, metrics: &[FunctionUnit], cfg: &Smells) -> Vec<SmellFinding> {
    // Performance smells need to look up callees by name to attribute
    // helper-in-loop work to the caller; same-named units stay ambiguous.
    let mut functions: BTreeMap<&str, Vec<&FunctionUnit>> = BTreeMap::new();
    for m in metrics {
        functions.entry(m.name.as_str()).or_default().push(m);
    }
    let profile = registry::performance_profile(ctx.language);
    let rules = LoopCallRules::new(cfg);
    metrics
        .iter()
        .flat_map(|m| {
            let mut findings = direct_findings(ctx, m, cfg, &functions, profile, &rules);
            findings.extend(helper_findings(ctx, m, cfg, &functions, profile, &rules));
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
    rules: &LoopCallRules,
) -> Vec<SmellFinding> {
    let mut out = Vec::new();
    let facts = &m.performance;
    let nested_loops: Vec<&PerfLine> = significant_loops(profile, &facts.nested_loops)
        .into_iter()
        .filter(|inner| {
            !walks_outer_item(facts, inner, registry::identifier_extra_chars(ctx.language))
        })
        .collect();
    if let Some(first) = nested_loops.first() {
        out.push(finding(
            SmellKind::NestedLoop,
            ctx,
            m,
            first.line,
            nested_loops.len(),
            &nested_loop_reason(facts, first),
            Severity::Warning,
        ));
    }
    let sorts: Vec<&PerfLine> = facts
        .sorts_in_loops
        .iter()
        .filter(|sort| !owned_by_loop(facts, sort.frame, profile.receiver_root(&sort.subject)))
        .collect();
    if let Some(first) = sorts.first() {
        out.push(finding(
            SmellKind::SortInLoop,
            ctx,
            m,
            first.line,
            sorts.len(),
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
    let per_item = facts
        .loop_calls
        .iter()
        .filter(|call| repeats_per_item(facts, call.frame, cfg.n_plus_one_max_constant_bound));
    for call in external_calls(ctx, m, per_item, functions, profile, rules).values() {
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
    cfg: &Smells,
    functions: &BTreeMap<&str, Vec<&FunctionUnit>>,
    profile: &dyn PerformanceProfile,
    rules: &LoopCallRules,
) -> Vec<SmellFinding> {
    let mut out = Vec::new();
    let facts = &m.performance;
    for call in &facts.loop_calls {
        if call.target == m.name {
            continue;
        }
        let Some(&[callee]) = functions.get(call.target.as_str()).map(Vec::as_slice) else {
            continue;
        };
        let callee_loops = significant_loops(profile, &callee.performance.loops);
        if !callee_loops.is_empty() && facts.in_loop_statement(call.frame) {
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
        if repeats_per_item(facts, call.frame, cfg.n_plus_one_max_constant_bound)
            && !external_calls(
                ctx,
                callee,
                &callee.performance.calls,
                functions,
                profile,
                rules,
            )
            .is_empty()
        {
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
        !call.base.is_empty() && (cfg.repeated_iteration_include_tests || !call.in_assertion)
    });
    for call in counted {
        by_base
            .entry((call.base.as_str(), call.region))
            .or_default()
            .push(call);
    }
    by_base
        .into_iter()
        .filter(|((base, _), calls)| {
            calls
                .iter()
                .any(|call| call.pass != IterationPass::Projection)
                && has_fusable_pair(m, profile, base, calls)
        })
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
    loops: &'a [PerfLine],
) -> Vec<&'a PerfLine> {
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
