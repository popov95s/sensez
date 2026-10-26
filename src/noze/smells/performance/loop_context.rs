use crate::profiles::typevocab::is_ident_char;
use crate::spine::ir::{LoopFrame, LoopKind, PerfLine, PerformanceFacts};

/// `name` is bound by, or declared inside, a loop enclosing `frame`.
pub(super) fn owned_by_loop(facts: &PerformanceFacts, frame: Option<usize>, name: &str) -> bool {
    facts.frames(frame).any(|f| f.owns(name))
}

/// Some enclosing loop neither has a small fixed trip count nor pages a cursor.
pub(super) fn repeats_per_item(
    facts: &PerformanceFacts,
    frame: Option<usize>,
    max_constant_bound: usize,
) -> bool {
    facts
        .frames(frame)
        .any(|f| !f.bound.within(max_constant_bound) && !f.is_paging())
}

/// Inner loop over a collection reached through an outer loop's item.
pub(super) fn walks_outer_item(facts: &PerformanceFacts, inner: &PerfLine, extra: &[char]) -> bool {
    let Some(own) = facts.frames(inner.frame).next() else {
        return false;
    };
    identifiers(&own.subject, extra).any(|ident| {
        facts
            .frames(own.parent)
            .any(|f| f.bindings.iter().any(|b| b == ident))
    })
}

pub(super) fn nested_loop_reason(facts: &PerformanceFacts, inner: &PerfLine) -> String {
    let mut frames = facts.frames(inner.frame);
    match (frames.next(), frames.next()) {
        (Some(own), Some(outer)) if iterates_items(own) && iterates_items(outer) => format!(
            "inner loop over `{}` walks an independent collection for each item of `{}`",
            own.subject, outer.subject
        ),
        _ => "nested loop multiplies work per input item".to_string(),
    }
}

fn iterates_items(frame: &LoopFrame) -> bool {
    frame.kind == LoopKind::ForOf && !frame.subject.is_empty()
}

/// Identifier tokens in `text` that are not a field access (`.x` / `?.x`).
fn identifiers<'a>(text: &'a str, extra: &'a [char]) -> impl Iterator<Item = &'a str> {
    let is_ident = |ch: char| is_ident_char(ch, extra);
    text.match_indices(move |ch: char| !is_ident(ch))
        .map(|(at, sep)| at + sep.len())
        .chain(std::iter::once(0))
        .filter_map(move |start| {
            let token = &text[start..];
            let len = token.find(|ch: char| !is_ident(ch)).unwrap_or(token.len());
            let field = text[..start].trim_end().ends_with('.');
            (len > 0 && !field).then(|| &token[..len])
        })
}
