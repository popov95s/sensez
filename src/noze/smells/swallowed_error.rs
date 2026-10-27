//! `swallowed_error_sentinel`: handlers that hide a failure behind a placeholder value or a log.

use super::{make, SmellContext};
use crate::config::smells::{Smells, Strictness};
use crate::report::{Severity, SmellFinding, SmellKind};
use crate::spine::ir::{FunctionUnit, GuardedBody, HandlerComment, HandlerEnd, SentinelHandler};

const OPTIONAL_PREFIXES: &[&str] = &["try", "safe", "maybe", "optional"];
const PREDICATE_PREFIXES: &[&str] = &["is", "has", "can", "should"];

pub(super) fn detect(
    ctx: &SmellContext<'_>,
    metric: &FunctionUnit,
    cfg: &Smells,
    out: &mut Vec<SmellFinding>,
) {
    if ctx.test_source || documents_sentinel(ctx, metric) {
        return;
    }
    let strictness = cfg.swallowed_error_strictness;
    let handlers = metric
        .review_risks
        .sentinel_handlers
        .iter()
        .filter(|handler| {
            reported(handler, strictness)
                && !retries(metric, handler)
                && !answers_predicate(metric, handler)
                && !explained(handler, strictness)
        });
    for handler in handlers {
        out.push(make(
            SmellKind::SwallowedErrorSentinel,
            message(ctx, handler),
            ctx.path,
            handler.line,
            &metric.name,
            Severity::Warning,
            1,
            0,
        ));
    }
}

fn reported(handler: &SentinelHandler, strictness: Strictness) -> bool {
    match (handler.end, strictness) {
        (HandlerEnd::EmptyValue, Strictness::Low) => handler.guarded == GuardedBody::Other,
        (HandlerEnd::EmptyValue, _) => true,
        (HandlerEnd::LogOnly | HandlerEnd::DefaultLiteral, Strictness::High) => true,
        (HandlerEnd::LogOnly | HandlerEnd::DefaultLiteral, _) => false,
    }
}

/// A log-only handler inside a loop statement lets the loop retry or skip the item.
fn retries(metric: &FunctionUnit, handler: &SentinelHandler) -> bool {
    handler.end == HandlerEnd::LogOnly && metric.performance.in_loop_statement(handler.frame)
}

/// `isValidUrl` answering `false` when parsing throws is the predicate's contract.
fn answers_predicate(metric: &FunctionUnit, handler: &SentinelHandler) -> bool {
    handler.end != HandlerEnd::LogOnly && named_with(&metric.name, PREDICATE_PREFIXES)
}

/// A commented log-only handler states why the failure is safe to ignore.
fn explained(handler: &SentinelHandler, strictness: Strictness) -> bool {
    handler.end == HandlerEnd::LogOnly
        && handler.comment == HandlerComment::Present
        && strictness != Strictness::High
}

/// `tryParse(): T | null` names and types its missing-value contract.
fn documents_sentinel(ctx: &SmellContext<'_>, metric: &FunctionUnit) -> bool {
    named_with(&metric.name, OPTIONAL_PREFIXES)
        && ctx
            .type_hints
            .return_type(metric)
            .is_some_and(|annotation| ctx.type_vocabulary.admits_missing(annotation))
}

/// `name` starts with one of `prefixes` as a whole word (`tryParse`, `is_valid`).
fn named_with(name: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|prefix| {
        name.strip_prefix(prefix)
            .and_then(|rest| rest.chars().next())
            .is_some_and(|next| next.is_uppercase() || next == '_')
    })
}

fn message(ctx: &SmellContext<'_>, handler: &SentinelHandler) -> String {
    let clause = ctx.wording.handler_clause;
    let advice = ctx.wording.swallowed_error_advice;
    match handler.end {
        HandlerEnd::LogOnly => format!(
            "{clause} at line {} only logs or ignores the error and carries on; {advice}",
            handler.line
        ),
        HandlerEnd::EmptyValue | HandlerEnd::DefaultLiteral => format!(
            "{clause} at line {} returns `{}` and drops the error, so callers get a plausible value instead of a failure; {advice}",
            handler.line, handler.literal
        ),
    }
}
