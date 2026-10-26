//! Shared loop-frame construction and trip counts.
#![cfg_attr(
    not(any(feature = "lang-python", feature = "lang-rust")),
    allow(dead_code)
)]

use super::node_query::{field_identifiers, loop_subject};
use crate::spine::ir::{LoopBound, LoopFrame, LoopKind};
use tree_sitter::Node;

/// One grammar's `for` loop: node kind, binding and subject fields, and how to read its bound.
pub(crate) struct ForLoopShape {
    pub(crate) kind: &'static str,
    pub(crate) binding_field: &'static str,
    pub(crate) subject_field: &'static str,
    pub(crate) trip_count: fn(&str) -> Option<usize>,
    pub(crate) is_bounded: fn(&str) -> bool,
}

/// Whether a range's end value is one of its iterations.
#[derive(Clone, Copy)]
pub(crate) enum RangeEnd {
    Exclusive,
    Inclusive,
}

/// Iterations of a unit-step range from `start` to `end`.
pub(crate) fn trip_count(start: usize, end: usize, range_end: RangeEnd) -> usize {
    match range_end {
        RangeEnd::Exclusive => end.saturating_sub(start),
        RangeEnd::Inclusive => end.saturating_add(1).saturating_sub(start),
    }
}

/// Frame for a `for` loop matching `shape`, else a condition loop.
pub(crate) fn loop_frame(
    node: Node,
    src: &[u8],
    parent: Option<usize>,
    shape: &ForLoopShape,
) -> LoopFrame {
    let text = |field| loop_subject(node, src, &[field]).unwrap_or_default();
    if node.kind() != shape.kind {
        return LoopFrame {
            kind: LoopKind::While,
            parent,
            subject: text("condition"),
            cond_idents: field_identifiers(node, src, "condition"),
            ..LoopFrame::default()
        };
    }
    let subject = text(shape.subject_field);
    LoopFrame {
        kind: LoopKind::ForOf,
        parent,
        bindings: field_identifiers(node, src, shape.binding_field),
        bound: bound(&subject, shape),
        subject,
        ..LoopFrame::default()
    }
}

fn bound(subject: &str, shape: &ForLoopShape) -> LoopBound {
    let compact: String = subject.chars().filter(|ch| !ch.is_whitespace()).collect();
    if let Some(count) = (shape.trip_count)(&compact) {
        return LoopBound::Count(count);
    }
    if (shape.is_bounded)(&compact) {
        return LoopBound::Fixed;
    }
    LoopBound::Unbounded
}
