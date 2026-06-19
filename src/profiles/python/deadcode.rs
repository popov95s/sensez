//! Python dead-code conventions: decorator-shape classification, private/test
//! naming, and entry-file stems.
//!
//! Decorators are classified by *shape* — not by a framework name list:
//! - **Registration**: hands the function to an external object, so it's
//!   reachable outside the import graph — `@app.get(...)`, `@router.websocket`,
//!   `@celery.task`, or any decorator whose trailing name the user listed in
//!   `entrypoints`. ⇒ treat as live.
//! - **Neutral**: structural/wrapping stdlib decorators — `@property`,
//!   `@functools.lru_cache`. ⇒ ignore.
//! - **Unknown**: a bare custom/unrecognized decorator (`@lru_cache`,
//!   `@my_wrapper`). ⇒ downgrade.

use crate::profiles::{DeadCodeDefaults, DecoratorClass};
use std::collections::HashSet;

const NEUTRAL_BASES: &[&str] = &[
    "functools",
    "abc",
    "typing",
    "dataclasses",
    "contextlib",
    "operator",
    "enum",
];

const NEUTRAL_NAMES: &[&str] = &[
    "property",
    "staticmethod",
    "classmethod",
    "abstractmethod",
    "abstractproperty",
    "dataclass",
    "cached_property",
    "overload",
    "override",
    "final",
    "wraps",
    "contextmanager",
    "asynccontextmanager",
];

pub fn defaults() -> DeadCodeDefaults {
    DeadCodeDefaults {
        entrypoints: &["route", "fixture", "task", "command", "app", "cli"],
        entrypoint_names: &["register", "main", "setup"],
        entrypoint_bases: &["AppConfig"],
        entry_points: &[
            "**/alembic/**",
            "**/migrations/**",
            "**/tests/**",
            "**/test/**",
            "**/conftest.py",
            "**/test_*.py",
            "**/*_test.py",
        ],
        test_sources: &[
            "**/tests/**",
            "**/test/**",
            "**/conftest.py",
            "**/test_*.py",
            "**/*_test.py",
        ],
    }
}

/// Classify decorator dotted-paths (e.g. `["app.get"]`, `["functools.lru_cache"]`).
pub fn classify(paths: Option<&Vec<String>>, user_entrypoints: &HashSet<String>) -> DecoratorClass {
    let paths = match paths {
        Some(p) if !p.is_empty() => p,
        _ => return DecoratorClass::None,
    };
    let mut unknown = false;
    for path in paths {
        let trailing = path.rsplit('.').next().unwrap_or(path);
        let head = path.split('.').next().unwrap_or(path);
        let is_attr = path.contains('.');

        if user_entrypoints.contains(trailing) {
            return DecoratorClass::Registration;
        }
        // Attribute-access decorator on a non-neutral object ⇒ registration.
        if is_attr && !NEUTRAL_BASES.contains(&head) {
            return DecoratorClass::Registration;
        }
        let neutral =
            NEUTRAL_NAMES.contains(&trailing) || (is_attr && NEUTRAL_BASES.contains(&head));
        if !neutral {
            unknown = true;
        }
    }
    if unknown {
        DecoratorClass::Unknown
    } else {
        DecoratorClass::Neutral
    }
}

/// Private/test by Python naming convention (leading `_`, `test_` prefix).
pub fn is_conventionally_private(symbol: &str) -> bool {
    symbol.starts_with('_') || symbol.starts_with("test_")
}

/// File stems that are always entry points (`__main__`).
pub fn is_entry_file_stem(stem: &str) -> bool {
    stem == "__main__"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eps() -> HashSet<String> {
        ["route", "fixture", "task"]
            .into_iter()
            .map(str::to_string)
            .collect()
    }

    fn classify_one(p: &str) -> DecoratorClass {
        classify(Some(&vec![p.to_string()]), &eps())
    }

    #[test]
    fn registration_shapes() {
        assert!(matches!(
            classify_one("app.get"),
            DecoratorClass::Registration
        ));
        assert!(matches!(
            classify_one("router.websocket"),
            DecoratorClass::Registration
        ));
        assert!(matches!(
            classify_one("pytest.fixture"),
            DecoratorClass::Registration
        ));
        assert!(matches!(classify_one("task"), DecoratorClass::Registration));
    }

    #[test]
    fn neutral_and_unknown() {
        assert!(matches!(classify_one("property"), DecoratorClass::Neutral));
        assert!(matches!(
            classify_one("functools.lru_cache"),
            DecoratorClass::Neutral
        ));
        assert!(matches!(classify_one("lru_cache"), DecoratorClass::Unknown));
        assert!(matches!(
            classify_one("my_wrapper"),
            DecoratorClass::Unknown
        ));
        assert!(matches!(classify(None, &eps()), DecoratorClass::None));
    }
}
