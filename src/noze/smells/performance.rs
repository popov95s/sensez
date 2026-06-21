//! Performance-oriented local smells.

use crate::config::smells::Smells;
use crate::noze::{Severity, SmellFinding, SmellKind};
use crate::spine::ir::{CallFact, FunctionUnit, PerfLine};
use crate::spine::parser::ParsedFile;
use std::collections::{BTreeMap, BTreeSet};

const EXPENSIVE_METHODS: [&str; 13] = [
    "all", "execute", "fetch", "fetchOne", "fetchone", "find", "findOne", "get", "load", "query",
    "request", "save", "select",
];

pub fn detect(file: &ParsedFile, _cfg: &Smells, out: &mut Vec<SmellFinding>) {
    let imports = import_bindings(file);
    let functions: BTreeMap<&str, &FunctionUnit> = file
        .walked
        .units
        .functions
        .iter()
        .map(|f| (f.name.as_str(), f))
        .collect();
    for function in &file.walked.units.functions {
        direct_findings(file, function, &imports, &functions, out);
        helper_findings(file, function, &imports, &functions, out);
    }
}

fn direct_findings(
    file: &ParsedFile,
    function: &FunctionUnit,
    imports: &BTreeSet<String>,
    functions: &BTreeMap<&str, &FunctionUnit>,
    out: &mut Vec<SmellFinding>,
) {
    push_line_finding(
        out,
        file,
        function,
        SmellKind::NestedLoop,
        &function.performance.nested_loops,
        "nested loop multiplies work per input item",
    );
    push_line_finding(
        out,
        file,
        function,
        SmellKind::SortInLoop,
        &function.performance.sorts_in_loops,
        "sort inside a loop repeats O(n log n) work",
    );
    for calls in repeated_iterations(function).values() {
        push_finding(
            out,
            SmellKind::RepeatedIteration,
            file,
            function,
            calls[0].line,
            calls.len(),
            "same collection is iterated multiple times in this scope",
            Severity::Warning,
        );
    }
    for call in external_calls(&function.performance.loop_calls, imports, functions).values() {
        push_finding(
            out,
            SmellKind::NPlusOneCall,
            file,
            function,
            call.line,
            1,
            "external-looking call runs once per loop iteration",
            Severity::Info,
        );
    }
}

fn helper_findings(
    file: &ParsedFile,
    function: &FunctionUnit,
    imports: &BTreeSet<String>,
    functions: &BTreeMap<&str, &FunctionUnit>,
    out: &mut Vec<SmellFinding>,
) {
    for call in &function.performance.loop_calls {
        let Some(callee) = functions.get(call.target.as_str()).copied() else {
            continue;
        };
        if !callee.performance.loops.is_empty() {
            push_finding(
                out,
                SmellKind::NestedLoop,
                file,
                function,
                call.line,
                callee.performance.loops.len() + 1,
                "helper called in a loop also iterates",
                Severity::Warning,
            );
        }
        if !external_calls(&callee.performance.calls, imports, functions).is_empty() {
            push_finding(
                out,
                SmellKind::NPlusOneCall,
                file,
                function,
                call.line,
                1,
                "helper called in a loop performs external-looking calls",
                Severity::Info,
            );
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
    imports: &BTreeSet<String>,
    functions: &BTreeMap<&str, &FunctionUnit>,
) -> BTreeMap<&'a str, &'a CallFact> {
    let mut out = BTreeMap::new();
    for call in calls {
        if functions.contains_key(call.target.as_str()) {
            continue;
        }
        if is_external(call, imports) {
            out.entry(call.target.as_str()).or_insert(call);
        }
    }
    out
}

fn push_line_finding(
    out: &mut Vec<SmellFinding>,
    file: &ParsedFile,
    function: &FunctionUnit,
    kind: SmellKind,
    lines: &[PerfLine],
    reason: &str,
) {
    if let Some(first) = lines.first() {
        push_finding(
            out,
            kind,
            file,
            function,
            first.line,
            lines.len(),
            reason,
            Severity::Warning,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn push_finding(
    out: &mut Vec<SmellFinding>,
    kind: SmellKind,
    file: &ParsedFile,
    function: &FunctionUnit,
    line: usize,
    metric: usize,
    reason: &str,
    severity: Severity,
) {
    out.push(finding(
        kind, file, function, line, metric, reason, severity,
    ));
}

fn is_external(call: &CallFact, imports: &BTreeSet<String>) -> bool {
    imports.contains(call.target.as_str())
        || (!call.base.is_empty() && imports.contains(call.base.as_str()))
        || (call.member && EXPENSIVE_METHODS.contains(&call.method.as_str()))
}

fn import_bindings(file: &ParsedFile) -> BTreeSet<String> {
    file.walked
        .symbols
        .imports
        .iter()
        .flat_map(|import| import.bindings.iter().chain(import.imported_symbols.iter()))
        .cloned()
        .collect()
}

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
