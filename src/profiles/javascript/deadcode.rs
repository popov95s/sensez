//! JS/TS dead-code conventions. Decorators (a TS feature) and Python-style
//! naming conventions don't apply; exported initializer callees stand in for
//! registration decorators. Full JS dead-code fidelity (barrel/re-export
//! forwarding) is a deferred milestone.

use crate::globs::matches_name;
use crate::profiles::{DeadCodeDefaults, DecoratorClass, ExternalBases};
use std::collections::HashSet;

/// Framework-invoked method callbacks: React lifecycle, custom elements, test-reporter hooks.
const CALLBACK_NAMES: &[&str] = &[
    "render",
    "componentDidMount",
    "componentDidUpdate",
    "componentWillUnmount",
    "shouldComponentUpdate",
    "getSnapshotBeforeUpdate",
    "componentDidCatch",
    "getDerivedStateFromError",
    "getDerivedStateFromProps",
    "connectedCallback",
    "disconnectedCallback",
    "attributeChangedCallback",
    "adoptedCallback",
    "onInit",
    "onBegin",
    "onEnd",
    "onTestBegin",
    "onTestEnd",
    "onTestResult",
    "onRunStart",
    "onRunComplete",
    "onFinished",
    "onStdOut",
    "onStdErr",
    "onError",
];

pub fn defaults() -> DeadCodeDefaults {
    DeadCodeDefaults {
        entrypoints: &[],
        entrypoint_names: &[],
        method_entrypoint_names: CALLBACK_NAMES,
        external_bases: ExternalBases::ImplyContract { except: &[] },
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
        test_sources: &[
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
        method_entrypoint_names: CALLBACK_NAMES,
        external_bases: ExternalBases::ImplyContract { except: &[] },
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
        test_sources: &[
            "**/__tests__/**",
            "**/tests/**",
            "**/test/**",
            "**/*.test.ts",
            "**/*.spec.ts",
            "**/*.test.tsx",
            "**/*.spec.tsx",
            "**/*.test.mts",
            "**/*.spec.mts",
            "**/*.test.cts",
            "**/*.spec.cts",
        ],
    }
}

/// `paths` are initializer callees (`export const x = register(...)`): a user-listed
/// trailing name marks a registration; anything else stays unclassified, never unknown.
pub fn classify(paths: Option<&Vec<String>>, user: &HashSet<String>) -> DecoratorClass {
    let registered = paths.is_some_and(|paths| {
        paths.iter().any(|path| {
            let trailing = path.rsplit('.').next().unwrap_or(path);
            user.iter().any(|pattern| matches_name(pattern, trailing))
        })
    });
    if registered {
        DecoratorClass::Registration
    } else {
        DecoratorClass::None
    }
}

/// No enforced private/test naming convention in JS/TS.
pub fn is_conventionally_private(_symbol: &str) -> bool {
    false
}

/// No universal entry-file stem (no `__main__`).
pub fn is_entry_file_stem(_stem: &str) -> bool {
    false
}
