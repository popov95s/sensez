//! Rust performance-smell profile policy.

use crate::spine::ir::PerfLine;

const EXPENSIVE_LOOP_METHODS: &[&str] = &[
    "execute", "fetch", "find", "load", "query", "read", "request", "save", "send",
];
const MUTATING_METHODS: &[&str] = &[
    "append",
    "clear",
    "dedup",
    "extend",
    "insert",
    "pop",
    "push",
    "remove",
    "retain",
    "reverse",
    "sort",
    "swap_remove",
    "truncate",
];
const STRONG_IO_METHODS: &[&str] = &["execute", "query", "request", "send"];
const EXTERNAL_RECEIVER_NAMES: &[&str] = &[
    "api",
    "client",
    "connection",
    "db",
    "repo",
    "repository",
    "session",
];
const EXTERNAL_TYPE_PARTS: &[&str] = &[
    "client",
    "connection",
    "database",
    "repository",
    "session",
    "transport",
];

pub(crate) fn is_mutating_call(method: &str) -> bool {
    MUTATING_METHODS.contains(&method)
}

pub(crate) fn receiver_root(receiver: &str) -> &str {
    receiver.split('.').next().unwrap_or(receiver)
}

pub(crate) fn is_bounded_loop(subject: &str) -> bool {
    let compact: String = subject.chars().filter(|ch| !ch.is_whitespace()).collect();
    is_constant_name(&compact)
        || is_small_literal_range(&compact)
        || has_literal_take(&compact)
        || is_literal_collection(&compact)
}

pub(crate) fn is_external_loop_call(
    method: &str,
    receiver: &str,
    receiver_type: Option<&str>,
    _loops: &[PerfLine],
) -> bool {
    if !EXPENSIVE_LOOP_METHODS.contains(&method) {
        return false;
    }
    if STRONG_IO_METHODS.contains(&method) {
        return true;
    }
    receiver_type.is_some_and(|type_name| {
        let lower = type_name.to_ascii_lowercase();
        EXTERNAL_TYPE_PARTS.iter().any(|part| lower.contains(part))
    }) || EXTERNAL_RECEIVER_NAMES
        .iter()
        .any(|word| receiver.to_ascii_lowercase().contains(word))
}

fn is_constant_name(subject: &str) -> bool {
    !subject.is_empty()
        && subject
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch == '_')
}

fn is_small_literal_range(subject: &str) -> bool {
    subject
        .split_once("..")
        .is_some_and(|(start, end)| start.parse::<i32>().is_ok() && end.parse::<i32>().is_ok())
}

fn has_literal_take(subject: &str) -> bool {
    subject
        .split(".take(")
        .nth(1)
        .and_then(|tail| tail.split(')').next())
        .and_then(|value| value.parse::<usize>().ok())
        .is_some_and(|value| value <= 100)
}

fn is_literal_collection(subject: &str) -> bool {
    subject.starts_with('[') && subject.ends_with(']') && subject.split(',').count() <= 12
}
