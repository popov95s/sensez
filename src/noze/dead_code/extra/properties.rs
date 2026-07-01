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
    let parsed_files: Vec<&ParsedFile> = files.into_iter().collect();
    let usage = ProjectUsage::from_files(&parsed_files);
    parsed_files
        .iter()
        .flat_map(|file| unused_properties_in_file(file, modmap, &usage))
        .collect()
}

fn unused_properties_in_file(
    file: &ParsedFile,
    modmap: &HashMap<PathBuf, String>,
    usage: &ProjectUsage,
) -> Vec<DeadCodeFinding> {
    let module = modmap.get(&file.path).cloned().unwrap_or_default();

    reportable_classes(file)
        .flat_map(|class| dead_properties_for_class(file, &module, usage, class))
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
    usage: &ProjectUsage,
    class: &ClassUnit,
) -> Vec<DeadCodeFinding> {
    class
        .properties
        .iter()
        .filter(|property| property_is_dead(usage, class, property))
        .map(|property| finding(file, module, class, property))
        .collect()
}

fn property_is_dead(usage: &ProjectUsage, class: &ClassUnit, property: &ClassProperty) -> bool {
    !property.name.starts_with('_')
        && !class_manages_all_fields(class)
        && !is_framework_managed_property(property)
        && !property_is_live(usage, class, property.name.as_str())
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

fn typed_receivers(file: &ParsedFile, class_names: &HashSet<String>) -> HashMap<String, String> {
    let hints = type_hints(file);
    let mut out = HashMap::new();
    for (name, ty) in &hints.var_types {
        if let Some((name, class)) = receiver_class(name, ty, class_names) {
            out.insert(name.to_string(), class);
        }
    }
    for ((_, name), ty) in &hints.param_types {
        if let Some((name, class)) = receiver_class(name, ty, class_names) {
            out.insert(name.to_string(), class);
        }
    }
    out
}

fn type_hints(file: &ParsedFile) -> &TypeHints {
    &file.walked.units.type_hints
}

fn receiver_class<'a>(
    name: &'a str,
    ty: &str,
    class_names: &HashSet<String>,
) -> Option<(&'a str, String)> {
    class_names
        .iter()
        .find(|class| type_mentions_class(ty, class))
        .map(|class| (name, class.clone()))
}

fn type_mentions_class(ty: &str, class: &str) -> bool {
    ty.rsplit(['.', '[', ']', ',', ' ', '|', ':', '<', '>', '('])
        .any(|part| part.trim_matches('?') == class)
}

fn untyped_attribute_names(
    file: &ParsedFile,
    receivers: &HashMap<String, String>,
) -> HashSet<String> {
    file.walked
        .usage
        .attribute_accesses
        .iter()
        .filter(|(base, _)| !receivers.contains_key(base.as_str()))
        .flat_map(|(_, attrs)| attrs.iter().cloned())
        .collect()
}

fn property_is_live(usage: &ProjectUsage, class: &ClassUnit, property: &str) -> bool {
    class
        .method_attr_use
        .values()
        .any(|attrs| attrs.contains(property))
        || usage
            .typed_property_uses
            .contains(&(class.name.clone(), property.to_string()))
        || usage
            .returned_property_uses
            .contains(&(class.name.clone(), property.to_string()))
        || usage.untyped_attrs.contains(property)
        || usage.chained_attrs.contains(property)
        || usage.string_literals.contains(property)
}

#[derive(Default)]
struct ProjectUsage {
    typed_property_uses: HashSet<(String, String)>,
    returned_property_uses: HashSet<(String, String)>,
    untyped_attrs: HashSet<String>,
    chained_attrs: HashSet<String>,
    string_literals: HashSet<String>,
}

impl ProjectUsage {
    fn from_files(files: &[&ParsedFile]) -> Self {
        let class_names = project_class_names(files);
        let mut usage = Self::default();
        for file in files {
            let receivers = typed_receivers(file, &class_names);
            usage
                .untyped_attrs
                .extend(untyped_attribute_names(file, &receivers));
            usage
                .chained_attrs
                .extend(file.walked.usage.chained_attribute_names.iter().cloned());
            usage
                .string_literals
                .extend(file.walked.usage.string_literals.iter().cloned());
            usage.collect_typed_property_uses(file, &receivers);
            usage.collect_returned_property_uses(file, &class_names);
        }
        usage
    }

    fn collect_typed_property_uses(
        &mut self,
        file: &ParsedFile,
        receivers: &HashMap<String, String>,
    ) {
        for (base, attrs) in &file.walked.usage.attribute_accesses {
            if let Some(class) = receivers.get(base) {
                self.typed_property_uses
                    .extend(attrs.iter().map(|attr| (class.clone(), attr.clone())));
            }
        }
    }

    fn collect_returned_property_uses(&mut self, file: &ParsedFile, class_names: &HashSet<String>) {
        for (function, attrs) in &file.walked.usage.call_result_attribute_accesses {
            if let Some(ty) = type_hints(file).return_types.get(function) {
                for class in class_names
                    .iter()
                    .filter(|class| type_mentions_class(ty, class))
                {
                    self.returned_property_uses
                        .extend(attrs.iter().map(|attr| (class.clone(), attr.clone())));
                }
            }
        }
    }
}

fn project_class_names(files: &[&ParsedFile]) -> HashSet<String> {
    files
        .iter()
        .flat_map(|file| file.walked.units.classes.iter())
        .map(|class| class.name.clone())
        .collect()
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
