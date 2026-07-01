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
use crate::spine::ir::{ClassProperty, ClassUnit};
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
            "**/docs/**",
            "**/docs_src/**",
            "**/examples/**",
            "**/example/**",
            "**/migrations/**",
            "**/tests/**",
            "**/test/**",
            "**/conftest.py",
            "**/test_*.py",
            "**/*_test.py",
        ],
        test_sources: &[
            "**/docs/**",
            "**/docs_src/**",
            "**/examples/**",
            "**/example/**",
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
    let decorator_paths = match paths {
        Some(p) if !p.is_empty() => p,
        _ => return DecoratorClass::None,
    };
    let mut unknown = false;
    for path in decorator_paths {
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

pub fn manages_class_properties(class: &ClassUnit, decorators: Option<&Vec<String>>) -> bool {
    class
        .bases
        .iter()
        .any(|base| is_external_field_container(base))
        || decorators.is_some_and(|paths| paths.iter().any(|d| is_data_class_decorator(d)))
}

pub fn manages_property(property: &ClassProperty) -> bool {
    property.name == "model_config"
        || schema_type_parts(&property.type_name).any(is_schema_field_type)
        || property
            .initializer_type
            .as_deref()
            .is_some_and(|ty| schema_type_parts(ty).any(is_schema_field_type))
}

pub fn requires_property_usage_evidence(class: &ClassUnit) -> bool {
    class.bases.iter().any(|base| {
        matches!(
            short_name(base),
            "BaseModel" | "GenericModel" | "BaseModelWithConfig"
        )
    })
}

fn is_external_field_container(base: &str) -> bool {
    matches!(
        short_name(base),
        "BaseConfig"
            | "BaseSettings"
            | "Model"
            | "Serializer"
            | "Schema"
            | "NamedTuple"
            | "TypedDict"
            | "Enum"
    )
}

fn is_data_class_decorator(path: &str) -> bool {
    matches!(
        short_name(path),
        "dataclass" | "define" | "frozen" | "s" | "mutable" | "immutable"
    )
}

fn schema_type_parts(text: &str) -> impl Iterator<Item = &str> {
    text.split(['[', ']', ',', ' ', '|', '.'])
}

fn is_schema_field_type(part: &str) -> bool {
    matches!(
        part.trim(),
        "Column" | "Mapped" | "Relationship" | "relationship"
    )
}

fn short_name(name: &str) -> &str {
    name.rsplit('.').next().unwrap_or(name)
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

    #[test]
    fn pydantic_config_members_are_framework_managed() {
        let property = ClassProperty {
            name: "model_config".to_string(),
            ..ClassProperty::default()
        };
        let class = ClassUnit {
            bases: vec!["pydantic.BaseConfig".to_string()],
            ..ClassUnit::default()
        };

        assert!(manages_property(&property));
        assert!(manages_class_properties(&class, None));
    }
}
