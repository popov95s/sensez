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
    let mut functions: BTreeMap<&str, Vec<&FunctionUnit>> = BTreeMap::new();
    for m in metrics {
        functions.entry(m.name.as_str()).or_default().push(m);
    }
    let scan = PerfScan {
        ctx,
        cfg,
        functions: &functions,
        profile: registry::performance_profile(ctx.language),
        rules: LoopCallRules::new(cfg),
    };
    metrics
        .iter()
        .flat_map(|m| {
            let mut findings: Vec<SmellFinding> = scan
                .nested_loop(m)
                .into_iter()
                .chain(scan.sort_in_loop(m))
                .collect();
            findings.extend(scan.repeated_iterations(m));
            findings.extend(scan.n_plus_one(m));
            findings.extend(scan.helper_findings(m));
            findings
        })
        .collect()
}

/// What every performance check of one file reads; same-named units stay ambiguous in `functions`.
struct PerfScan<'a> {
    ctx: &'a SmellContext<'a>,
    cfg: &'a Smells,
    functions: &'a BTreeMap<&'a str, Vec<&'a FunctionUnit>>,
    profile: &'static dyn PerformanceProfile,
    rules: LoopCallRules,
}

impl PerfScan<'_> {
    fn identifier_chars(&self) -> &'static [char] {
        registry::identifier_extra_chars(self.ctx.language)
    }

    fn nested_loop(&self, m: &FunctionUnit) -> Option<SmellFinding> {
        let facts = &m.performance;
        let nested_loops: Vec<&PerfLine> = significant_loops(self.profile, &facts.nested_loops)
            .into_iter()
            .filter(|inner| !walks_outer_item(facts, inner, self.identifier_chars()))
            .collect();
        let first = nested_loops.first()?;
        Some(finding(
            SmellKind::NestedLoop,
            self.ctx,
            m,
            first.line,
            nested_loops.len(),
            &nested_loop_reason(facts, first),
            Severity::Warning,
        ))
    }

    fn sort_in_loop(&self, m: &FunctionUnit) -> Option<SmellFinding> {
        let facts = &m.performance;
        let sorts: Vec<&PerfLine> = facts
            .sorts_in_loops
            .iter()
            .filter(|sort| {
                !owned_by_loop(facts, sort.frame, self.profile.receiver_root(&sort.subject))
            })
            .collect();
        let first = sorts.first()?;
        Some(finding(
            SmellKind::SortInLoop,
            self.ctx,
            m,
            first.line,
            sorts.len(),
            "sort inside a loop repeats O(n log n) work",
            Severity::Warning,
        ))
    }

    fn repeated_iterations(&self, m: &FunctionUnit) -> Vec<SmellFinding> {
        let in_tests = self.ctx.test_source || m.review_risks.is_test_case;
        if in_tests && !self.cfg.repeated_iteration_include_tests {
            return Vec::new();
        }
        repeated_iterations(m, self.cfg, self.profile)
            .into_iter()
            .map(|calls| {
                finding(
                    SmellKind::RepeatedIteration,
                    self.ctx,
                    m,
                    calls[0].line,
                    calls.len(),
                    &repeated_iteration_reason(&calls),
                    Severity::Warning,
                )
            })
            .collect()
    }

    fn n_plus_one(&self, m: &FunctionUnit) -> Vec<SmellFinding> {
        let facts = &m.performance;
        let bound = self.cfg.n_plus_one_max_constant_bound;
        let per_item = facts
            .loop_calls
            .iter()
            .filter(|call| repeats_per_item(facts, call.frame, bound));
        self.external_calls(m, per_item)
            .values()
            .map(|call| {
                finding(
                    SmellKind::NPlusOneCall,
                    self.ctx,
                    m,
                    call.line,
                    1,
                    "external-looking call runs once per loop iteration",
                    Severity::Info,
                )
            })
            .collect()
    }

    fn helper_findings(&self, m: &FunctionUnit) -> Vec<SmellFinding> {
        let mut out = Vec::new();
        let facts = &m.performance;
        for call in &facts.loop_calls {
            if call.target == m.name {
                continue;
            }
            let Some(&[callee]) = self.functions.get(call.target.as_str()).map(Vec::as_slice)
            else {
                continue;
            };
            let callee_loops = significant_loops(self.profile, &callee.performance.loops);
            if !callee_loops.is_empty() && facts.in_loop_statement(call.frame) {
                out.push(finding(
                    SmellKind::NestedLoop,
                    self.ctx,
                    m,
                    call.line,
                    callee_loops.len() + 1,
                    "helper called in a loop also iterates",
                    Severity::Warning,
                ));
            }
            if repeats_per_item(facts, call.frame, self.cfg.n_plus_one_max_constant_bound)
                && !self
                    .external_calls(callee, &callee.performance.calls)
                    .is_empty()
            {
                out.push(finding(
                    SmellKind::NPlusOneCall,
                    self.ctx,
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

    fn external_calls<'c>(
        &self,
        owner: &FunctionUnit,
        calls: impl IntoIterator<Item = &'c CallFact>,
    ) -> BTreeMap<&'c str, &'c CallFact> {
        external_calls(
            self.ctx,
            owner,
            calls,
            self.functions,
            self.profile,
            &self.rules,
        )
    }
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
            worth_fusing(calls) && has_fusable_pair(m, profile, base, calls)
        })
        .map(|(_, calls)| calls)
        .collect()
}

/// Three real passes, two loops, or two passes where one aggregates; projections do not count.
fn worth_fusing(calls: &[&CallFact]) -> bool {
    let count = |pass: IterationPass| calls.iter().filter(|call| call.pass == pass).count();
    let passes = calls.len() - count(IterationPass::Projection);
    passes >= 3
        || count(IterationPass::Loop) >= 2
        || (passes >= 2 && count(IterationPass::Aggregate) >= 1)
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
