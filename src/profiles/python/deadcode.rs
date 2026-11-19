//! Python dead-code conventions: decorator shape, private naming, entry-file stems.

use crate::globs::matches_any;
use crate::profiles::walk;
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
        registration_decorators: &["route", "fixture", "task", "command", "app", "cli"],
        entrypoint_names: &["register", "main", "setup"],
        entrypoint_bases: &["AppConfig"],
        entry_globs: &["**/alembic/**", "**/migrations/**"],
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
        ..DeadCodeDefaults::EMPTY
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
        let trailing = walk::last_segment(path);
        let head = path.split('.').next().unwrap_or(path);
        let is_attr = path.contains('.');

        if matches_any(user_entrypoints, trailing) {
            return DecoratorClass::Registration;
        }
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
            walk::last_segment(base),
            "BaseModel" | "GenericModel" | "BaseModelWithConfig"
        )
    })
}

fn is_external_field_container(base: &str) -> bool {
    matches!(
        walk::last_segment(base),
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
        walk::last_segment(path),
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

#[cfg(test)]
#[path = "deadcode_tests.rs"]
mod tests;
