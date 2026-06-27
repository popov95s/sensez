//! Rust performance-smell profile policy.

pub(crate) const EXPENSIVE_LOOP_METHODS: &[&str] = &[
    "execute", "fetch", "find", "get", "load", "query", "read", "request", "save", "send",
];
pub(crate) const EXTERNAL_GET_RECEIVERS: &[&str] = &[
    "api",
    "client",
    "conn",
    "connection",
    "cursor",
    "db",
    "repo",
    "repository",
    "session",
];
