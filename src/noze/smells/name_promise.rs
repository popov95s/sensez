//! `name_promise_mismatch`: a read-named function that writes state.

use super::name_promise_writes::{normalized, writes, Body, Reach, Write};
use super::{make, SmellContext};
use crate::config::smells::{Smells, Strictness};
use crate::profiles::registry;
use crate::report::{Severity, SmellFinding, SmellKind};
use crate::spine::ir::{CallbackPosition, FunctionUnit};

const WRITE_NAMES: &[&str] = &[
    "orcreate", "orinsert", "orset", "orupdate", "ensure", "upsert",
];
const HIGH_READ_PREFIXES: &[&str] = &["load", "fetch"];
const VALIDATION_PREFIXES: &[&str] = &["validate", "check"];
const SHOWN_WRITES: usize = 2;

pub fn detect(
    ctx: &SmellContext<'_>,
    metrics: &[FunctionUnit],
    cfg: &Smells,
    out: &mut Vec<SmellFinding>,
) {
    if ctx.test_source || !cfg.is_enabled(SmellKind::NamePromiseMismatch) {
        return;
    }
    let profile = registry::performance_profile(ctx.language);
    let private_prefixes = registry::smell_wording(ctx.language).private_name_prefixes;
    for unit in metrics.iter().filter(|unit| is_named_read_candidate(unit)) {
        let Some(prefix) = read_prefix(unit.name.trim_start_matches(private_prefixes), cfg) else {
            continue;
        };
        let body = Body {
            unit,
            callbacks: inline_callbacks(unit, metrics),
            enclosing: enclosing(unit, metrics),
        };
        let floor = reach_floor(&prefix, cfg.name_promise_strictness);
        let found: Vec<Write> = writes(&body, profile, cfg)
            .into_iter()
            .filter(|write| write.reach >= floor)
            .collect();
        if !found.is_empty() {
            out.push(finding(ctx, unit, &found));
        }
    }
}

fn is_named_read_candidate(unit: &FunctionUnit) -> bool {
    unit.callback_position == CallbackPosition::Standalone
        && !unit.is_accessor
        && !unit.is_constructor
        && !unit.review_risks.is_test_case
        && !names_a_write(&unit.name)
}

/// `getOrCreateUser`, `ensureUser`, `checkAndNotify`: the name already admits a write.
fn names_a_write(name: &str) -> bool {
    let conjoined = name.contains("_and_")
        || name
            .match_indices("And")
            .any(|(at, _)| name[at + 3..].starts_with(char::is_uppercase));
    conjoined
        || WRITE_NAMES
            .iter()
            .any(|word| normalized(name).contains(word))
}

/// The read prefix `bare` starts with, followed by a word boundary.
fn read_prefix(bare: &str, cfg: &Smells) -> Option<String> {
    let high = HIGH_READ_PREFIXES
        .iter()
        .filter(|_| cfg.name_promise_strictness == Strictness::High)
        .map(|prefix| prefix.to_string());
    cfg.name_promise_read_prefixes
        .iter()
        .cloned()
        .chain(high)
        .find(|prefix| {
            bare.strip_prefix(prefix.as_str())
                .and_then(|rest| rest.chars().next())
                .is_some_and(|next| next.is_uppercase() || next == '_')
        })
}

/// The least-reaching write that counts for this prefix and strictness.
fn reach_floor(prefix: &str, strictness: Strictness) -> Reach {
    if VALIDATION_PREFIXES.contains(&prefix) {
        return Reach::Persistent;
    }
    match strictness {
        Strictness::Low => Reach::Persistent,
        Strictness::Medium => Reach::Shared,
        Strictness::High => Reach::Cached,
    }
}

/// Callbacks passed inline anywhere inside `unit`'s body.
fn inline_callbacks<'a>(unit: &FunctionUnit, metrics: &'a [FunctionUnit]) -> Vec<&'a FunctionUnit> {
    metrics
        .iter()
        .filter(|other| {
            other.callback_position == CallbackPosition::Argument && contains(unit, other)
        })
        .collect()
}

/// Functions whose bodies hold `unit`.
fn enclosing<'a>(unit: &FunctionUnit, metrics: &'a [FunctionUnit]) -> Vec<&'a FunctionUnit> {
    metrics
        .iter()
        .filter(|outer| contains(outer, unit))
        .collect()
}

fn contains(outer: &FunctionUnit, inner: &FunctionUnit) -> bool {
    let same = outer.start_line == inner.start_line && outer.end_line == inner.end_line;
    !same && outer.start_line <= inner.start_line && inner.end_line <= outer.end_line
}

fn finding(ctx: &SmellContext<'_>, unit: &FunctionUnit, found: &[Write]) -> SmellFinding {
    let shown: Vec<String> = found
        .iter()
        .take(SHOWN_WRITES)
        .map(|write| format!("`{}` at L{}", write.target, write.line))
        .collect();
    let more = match found.len().saturating_sub(SHOWN_WRITES) {
        0 => String::new(),
        extra => format!(" (+{extra} more)"),
    };
    let finding = make(
        SmellKind::NamePromiseMismatch,
        format!(
            "`{}` writes: {}{more} — rename it to say so or move the write to the caller",
            unit.name,
            shown.join(", ")
        ),
        ctx.path,
        unit.start_line,
        &unit.name,
        Severity::Info,
        found.len() as u32,
        0,
    );
    SmellFinding {
        lines: found.iter().map(|write| write.line).collect(),
        ..finding
    }
}
