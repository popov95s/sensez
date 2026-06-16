//! JS/TS dead-code conventions. Decorators (a TS feature) and Python-style
//! naming conventions don't apply, so these are graceful no-ops — the generic
//! reachability pass (inbound import usage) still runs. Full JS dead-code
//! fidelity (barrel/re-export forwarding) is a deferred milestone.

use crate::profiles::{DeadCodeDefaults, DecoratorClass};
use std::collections::HashSet;

pub fn defaults() -> DeadCodeDefaults {
    DeadCodeDefaults {
        entrypoints: &[],
        entrypoint_names: &[],
        entrypoint_bases: &[],
        entry_points: &[
            "**/__tests__/**",
            "**/tests/**",
            "**/test/**",
            "**/*.test.js",
            "**/*.spec.js",
            "**/*.test.jsx",
            "**/*.spec.jsx",
        ],
    }
}

pub fn typescript_defaults() -> DeadCodeDefaults {
    DeadCodeDefaults {
        entrypoints: &[],
        entrypoint_names: &[],
        entrypoint_bases: &[],
        entry_points: &[
            "**/__tests__/**",
            "**/tests/**",
            "**/test/**",
            "**/*.test.ts",
            "**/*.spec.ts",
            "**/*.test.tsx",
            "**/*.spec.tsx",
        ],
    }
}

/// JS has no reachability-affecting decorators in scope → never classified.
pub fn classify(_paths: Option<&Vec<String>>, _user: &HashSet<String>) -> DecoratorClass {
    DecoratorClass::None
}

/// No enforced private/test naming convention in JS/TS.
pub fn is_conventionally_private(_symbol: &str) -> bool {
    false
}

/// No universal entry-file stem (no `__main__`).
pub fn is_entry_file_stem(_stem: &str) -> bool {
    false
}
