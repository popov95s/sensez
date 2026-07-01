//! Opt-in dead-property detection.
//!
//! This pass is class-aware: a property is live when its owning class methods
//! use it, when a typed receiver uses it, when the same file has an untyped
//! attribute access to that name, or when the name appears as a string literal
//! for dynamic APIs/serializers.

use crate::report::{ActionLevel, Confidence, DeadCodeFinding};
use crate::spine::ir::{ClassProperty, ClassUnit, TypeHints};
use crate::spine::parser::{ParsedFile, SymbolKind};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

pub fn unused_properties<'a>(
    files: impl IntoIterator<Item = &'a ParsedFile>,
    modmap: &HashMap<PathBuf, String>,
) -> Vec<DeadCodeFinding> {
    files
        .into_iter()
        .flat_map(|file| unused_properties_in_file(file, modmap))
        .collect()
}

fn unused_properties_in_file(
    file: &ParsedFile,
    modmap: &HashMap<PathBuf, String>,
) -> Vec<DeadCodeFinding> {
    let module = modmap.get(&file.path).cloned().unwrap_or_default();
    let class_names = class_names(file);
    let receivers = typed_receivers(file, &class_names);
    let untyped_attrs = untyped_attribute_names(file, &receivers);

    reportable_classes(file)
        .flat_map(|class| {
            dead_properties_for_class(file, &module, &receivers, &untyped_attrs, class)
        })
        .collect()
}

fn reportable_classes(file: &ParsedFile) -> impl Iterator<Item = &ClassUnit> {
    file.walked
        .units
        .classes
        .iter()
        .filter(|class| !class.properties.is_empty())
}

fn dead_properties_for_class(
    file: &ParsedFile,
    module: &str,
    receivers: &HashMap<&str, &str>,
    untyped_attrs: &HashSet<&str>,
    class: &ClassUnit,
) -> Vec<DeadCodeFinding> {
    class
        .properties
        .iter()
        .filter(|property| property_is_dead(file, receivers, untyped_attrs, class, property))
        .map(|property| finding(file, module, class, property))
        .collect()
}

fn property_is_dead(
    file: &ParsedFile,
    receivers: &HashMap<&str, &str>,
    untyped_attrs: &HashSet<&str>,
    class: &ClassUnit,
    property: &ClassProperty,
) -> bool {
    !property.name.starts_with('_')
        && !class_manages_all_fields(class)
        && !is_framework_managed_property(property)
        && !property_is_live(
            file,
            receivers,
            untyped_attrs,
            class,
            property.name.as_str(),
        )
}

fn finding(
    file: &ParsedFile,
    module: &str,
    class: &ClassUnit,
    property: &ClassProperty,
) -> DeadCodeFinding {
    DeadCodeFinding {
        action: ActionLevel::Advisory,
        module: module.to_string(),
        symbol: format!("{}.{}", class.name, property.name),
        kind: SymbolKind::Property,
        confidence: Confidence::Low,
        file: file.path.clone(),
        line: property.line,
        reason: String::new(),
    }
}

fn class_names(file: &ParsedFile) -> HashSet<&str> {
    file.walked
        .units
        .classes
        .iter()
        .map(|class| class.name.as_str())
        .collect()
}

fn typed_receivers<'a>(
    file: &'a ParsedFile,
    class_names: &HashSet<&'a str>,
) -> HashMap<&'a str, &'a str> {
    let hints = type_hints(file);
    let mut out = HashMap::new();
    for (name, ty) in &hints.var_types {
        if let Some((name, class)) = receiver_class(name, ty, class_names) {
            out.insert(name, class);
        }
    }
    for ((_, name), ty) in &hints.param_types {
        if let Some((name, class)) = receiver_class(name, ty, class_names) {
            out.insert(name, class);
        }
    }
    out
}

fn type_hints(file: &ParsedFile) -> &TypeHints {
    &file.walked.units.type_hints
}

fn receiver_class<'a>(
    name: &'a str,
    ty: &'a str,
    class_names: &HashSet<&'a str>,
) -> Option<(&'a str, &'a str)> {
    class_names
        .iter()
        .copied()
        .find(|class| type_mentions_class(ty, class))
        .map(|class| (name, class))
}

fn type_mentions_class(ty: &str, class: &str) -> bool {
    ty.rsplit(['.', '[', ']', ',', ' ', '|', ':', '<', '>', '('])
        .any(|part| part.trim_matches('?') == class)
}

fn untyped_attribute_names<'a>(
    file: &'a ParsedFile,
    receivers: &HashMap<&str, &str>,
) -> HashSet<&'a str> {
    file.walked
        .usage
        .attribute_accesses
        .iter()
        .filter(|(base, _)| !receivers.contains_key(base.as_str()))
        .flat_map(|(_, attrs)| attrs.iter().map(String::as_str))
        .collect()
}

fn property_is_live(
    file: &ParsedFile,
    receivers: &HashMap<&str, &str>,
    untyped_attrs: &HashSet<&str>,
    class: &ClassUnit,
    property: &str,
) -> bool {
    class
        .method_attr_use
        .values()
        .any(|attrs| attrs.contains(property))
        || typed_receiver_uses_property(file, receivers, class.name.as_str(), property)
        || returned_value_uses_property(file, class.name.as_str(), property)
        || untyped_attrs.contains(property)
        || file.walked.usage.chained_attribute_names.contains(property)
        || file.walked.usage.string_literals.contains(property)
}

fn typed_receiver_uses_property(
    file: &ParsedFile,
    receivers: &HashMap<&str, &str>,
    class_name: &str,
    property: &str,
) -> bool {
    file.walked
        .usage
        .attribute_accesses
        .iter()
        .filter(|(base, _)| receivers.get(base.as_str()) == Some(&class_name))
        .any(|(_, attrs)| attrs.contains(property))
}

fn returned_value_uses_property(file: &ParsedFile, class_name: &str, property: &str) -> bool {
    file.walked
        .usage
        .call_result_attribute_accesses
        .iter()
        .any(|(function, attrs)| {
            attrs.contains(property)
                && type_hints(file)
                    .return_types
                    .get(function)
                    .is_some_and(|ty| type_mentions_class(ty, class_name))
        })
}

fn class_manages_all_fields(class: &ClassUnit) -> bool {
    class
        .bases
        .iter()
        .any(|base| is_external_field_container(base.as_str()))
}

#[cfg(test)]
#[path = "properties_tests.rs"]
mod tests;

fn is_external_field_container(base: &str) -> bool {
    matches!(
        short_name(base),
        "BaseSettings" | "NamedTuple" | "TypedDict"
    )
}

fn is_framework_managed_property(property: &ClassProperty) -> bool {
    schema_type_parts(&property.type_name).any(is_schema_field_type)
        || property
            .initializer_type
            .as_deref()
            .is_some_and(|ty| schema_type_parts(ty).any(is_schema_field_type))
}

fn schema_type_parts(text: &str) -> impl Iterator<Item = &str> {
    text.split(['[', ']', ',', ' ', '|', '.'])
}

fn is_schema_field_type(part: &str) -> bool {
    matches!(
        part.trim(),
        "Column" | "Field" | "Mapped" | "Relationship" | "relationship"
    )
}

fn short_name(name: &str) -> &str {
    name.rsplit('.').next().unwrap_or(name)
}
