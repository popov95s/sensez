//! Performance-oriented local smells.

use crate::config::smells::Smells;
use crate::noze::{Severity, SmellFinding, SmellKind};
use crate::spine::ir::{CallFact, FunctionUnit, PerfLine};
use crate::spine::parser::ParsedFile;
use std::collections::BTreeMap;

const EXPENSIVE_METHODS: [&str; 12] = [
    "all", "execute", "fetch", "fetchOne", "fetchone", "find", "findOne", "load", "query",
    "request", "save", "select",
];
const EXPENSIVE_GET_RECEIVERS: [&str; 10] = [
    // TODO: Make per language
    "api",
    "client",
    "conn",
    "connection",
    "cursor",
    "db",
    "repo",
    "repository",
    "requests",
    "session",
];

pub fn detect(file: &ParsedFile, _cfg: &Smells, out: &mut Vec<SmellFinding>) {
    let functions: BTreeMap<&str, &FunctionUnit> = file
        .walked
        .units
        .functions
        .iter()
        .map(|f| (f.name.as_str(), f))
        .collect();
    for function in &file.walked.units.functions {
        direct_findings(file, function, &functions, out);
        helper_findings(file, function, &functions, out);
    }
}

fn direct_findings(
    file: &ParsedFile,
    function: &FunctionUnit,
    functions: &BTreeMap<&str, &FunctionUnit>,
    out: &mut Vec<SmellFinding>,
) {
    let nested_loops = significant_loops(&function.performance.nested_loops);
    if let Some(first) = nested_loops.first() {
        out.push(finding(
            SmellKind::NestedLoop,
            file,
            function,
            first.line,
            nested_loops.len(),
            "nested loop multiplies work per input item",
            Severity::Warning,
        ));
    }
    if let Some(first) = function.performance.sorts_in_loops.first() {
        out.push(finding(
            SmellKind::SortInLoop,
            file,
            function,
            first.line,
            function.performance.sorts_in_loops.len(),
            "sort inside a loop repeats O(n log n) work",
            Severity::Warning,
        ));
    }
    for calls in repeated_iterations(function).values() {
        out.push(finding(
            SmellKind::RepeatedIteration,
            file,
            function,
            calls[0].line,
            calls.len(),
            "same collection is iterated multiple times in this scope",
            Severity::Warning,
        ));
    }
    for call in external_calls(&function.performance.loop_calls, functions).values() {
        out.push(finding(
            SmellKind::NPlusOneCall,
            file,
            function,
            call.line,
            1,
            "external-looking call runs once per loop iteration",
            Severity::Info,
        ));
    }
}

fn helper_findings(
    file: &ParsedFile,
    function: &FunctionUnit,
    functions: &BTreeMap<&str, &FunctionUnit>,
    out: &mut Vec<SmellFinding>,
) {
    for call in &function.performance.loop_calls {
        let Some(callee) = functions.get(call.target.as_str()).copied() else {
            continue;
        };
        let callee_loops = significant_loops(&callee.performance.loops);
        if !callee_loops.is_empty() {
            out.push(finding(
                SmellKind::NestedLoop,
                file,
                function,
                call.line,
                callee_loops.len() + 1,
                "helper called in a loop also iterates",
                Severity::Warning,
            ));
        }
        if !external_calls(&callee.performance.calls, functions).is_empty() {
            out.push(finding(
                SmellKind::NPlusOneCall,
                file,
                function,
                call.line,
                1,
                "helper called in a loop performs external-looking calls",
                Severity::Info,
            ));
        }
    }
}

fn repeated_iterations(function: &FunctionUnit) -> BTreeMap<&str, Vec<&CallFact>> {
    let mut by_base: BTreeMap<&str, Vec<&CallFact>> = BTreeMap::new();
    for call in &function.performance.iteration_calls {
        if !call.base.is_empty() {
            by_base.entry(call.base.as_str()).or_default().push(call);
        }
    }
    by_base.retain(|_, calls| calls.len() > 1);
    by_base
}

fn external_calls<'a>(
    calls: &'a [CallFact],
    functions: &BTreeMap<&str, &FunctionUnit>,
) -> BTreeMap<&'a str, &'a CallFact> {
    let mut out = BTreeMap::new();
    for call in calls {
        if functions.contains_key(call.target.as_str()) {
            continue;
        }
        if is_external(call) {
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
    file: &ParsedFile,
    function: &FunctionUnit,
    line: usize,
    metric: usize,
    reason: &str,
    severity: Severity,
) -> SmellFinding {
    super::make(
        kind,
        format!("{reason}; combine the work or use a bulk operation."),
        &file.path,
        line,
        &function.name,
        severity,
        metric as u32,
        1,
    )
}

fn is_external(call: &CallFact) -> bool {
    call.member
        && (EXPENSIVE_METHODS.contains(&call.method.as_str())
            || (call.method == "get" && looks_external_receiver(&call.base)))
}

fn looks_external_receiver(base: &str) -> bool {
    EXPENSIVE_GET_RECEIVERS.contains(&base)
}

fn is_bounded_constant(subject: &str) -> bool {
    !subject.is_empty()
        && subject
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch == '_')
}
