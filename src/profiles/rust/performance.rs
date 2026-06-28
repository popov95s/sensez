//! Rust performance-smell profile policy.

pub(crate) const EXPENSIVE_LOOP_METHODS: &[&str] = &[
    "execute", "fetch", "find", "load", "query", "read", "request", "save", "send",
];
