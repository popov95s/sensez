//! `magic_string_default`: string fallbacks standing in for a missing value.

use super::{make, SmellContext};
use crate::config::smells::{Smells, Strictness};
use crate::report::{Severity, SmellFinding, SmellKind};
use crate::spine::ir::{FallbackSink, FunctionUnit, StringFallback};

const TYPOGRAPHIC: &[&str] = &["—", "–", "·", "…"];

pub(super) fn detect(
    ctx: &SmellContext<'_>,
    m: &FunctionUnit,
    cfg: &Smells,
    out: &mut Vec<SmellFinding>,
) {
    let hits: Vec<&StringFallback> = m
        .string_fallbacks
        .iter()
        .filter(|fallback| is_sentinel(ctx, fallback, cfg.magic_string_default_strictness))
        .collect();
    let Some(first) = hits.first() else {
        return;
    };
    out.push(make(
        SmellKind::MagicStringDefault,
        format!(
            "{} fallback string sentinel(s) (first: `{:?}` {} at line {}) — hidden sentinel values like {}; {}",
            hits.len(),
            first.literal,
            sink_phrase(first.sink),
            first.line,
            ctx.wording.fallback_example,
            ctx.wording.fallback_advice,
        ),
        ctx.path,
        first.line,
        &m.name,
        Severity::Warning,
        hits.len() as u32,
        0,
    ));
}

fn is_sentinel(ctx: &SmellContext<'_>, fallback: &StringFallback, strictness: Strictness) -> bool {
    let short = fallback.literal.chars().count() <= 1;
    let wordy = ctx
        .wording
        .sentinel_words
        .iter()
        .any(|word| fallback.literal.eq_ignore_ascii_case(word))
        || is_bracketed(&fallback.literal);
    let storing = matches!(fallback.sink, FallbackSink::Stored | FallbackSink::Compared);
    match (strictness, fallback.sink) {
        (_, FallbackSink::Unclassified) => short || wordy,
        (Strictness::Low, _) => storing && (short || wordy),
        (Strictness::Medium, FallbackSink::Rendered | FallbackSink::StringOp) => false,
        (Strictness::Medium, _) if TYPOGRAPHIC.contains(&fallback.literal.as_str()) => storing,
        (Strictness::Medium, FallbackSink::Returned | FallbackSink::Other) => short,
        (Strictness::Medium, _) | (Strictness::High, _) => short || wordy,
    }
}

const BRACKETS: [(char, char); 3] = [('(', ')'), ('<', '>'), ('[', ']')];

fn is_bracketed(literal: &str) -> bool {
    literal.chars().count() <= 24
        && BRACKETS
            .iter()
            .any(|(open, close)| literal.starts_with(*open) && literal.ends_with(*close))
}

fn sink_phrase(sink: FallbackSink) -> &'static str {
    match sink {
        FallbackSink::Rendered => "rendered",
        FallbackSink::StringOp => "passed to a string operation",
        FallbackSink::Stored => "stored or sent",
        FallbackSink::Compared => "compared",
        FallbackSink::Returned => "returned",
        FallbackSink::Other | FallbackSink::Unclassified => "used",
    }
}
