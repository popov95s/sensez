//! Opt-in dead-property detection.
//!
//! This pass is class-aware: a property is live when its owning class methods
//! use it, when a typed receiver uses it, when the same file has an untyped
//! attribute access to that name, or when the name appears as a string literal
//! for dynamic APIs/serializers.

use super::usage::ProjectUsage;
use crate::profiles::{registry, DeadCodeProfile};
use crate::report::{ActionLevel, Confidence, DeadCodeFinding};
use crate::spine::ir::{ClassProperty, ClassUnit};
use crate::spine::parser::{ParsedFile, SymbolKind};
use std::collections::HashMap;
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
    let profile = registry::dead_code_profile(file.language);

    reportable_classes(file)
        .flat_map(|class| dead_properties_for_class(file, &module, usage, profile, class))
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
    profile: &dyn DeadCodeProfile,
    class: &ClassUnit,
) -> Vec<DeadCodeFinding> {
    class
        .properties
        .iter()
        .filter(|property| property_is_dead(file, usage, profile, class, property))
        .map(|property| finding(file, module, usage, class, property))
        .collect()
}

fn property_is_dead(
    file: &ParsedFile,
    usage: &ProjectUsage,
    profile: &dyn DeadCodeProfile,
    class: &ClassUnit,
    property: &ClassProperty,
) -> bool {
    !property.name.starts_with('_')
        && !class_manages_all_fields(file, profile, class)
        && !profile.manages_property(property)
        && (!profile.requires_property_usage_evidence(class)
            || usage.class_has_member_evidence(&class.name))
        && !property_is_live(usage, class, property.name.as_str())
}

fn finding(
    file: &ParsedFile,
    module: &str,
    usage: &ProjectUsage,
    class: &ClassUnit,
    property: &ClassProperty,
) -> DeadCodeFinding {
    DeadCodeFinding {
        action: ActionLevel::Advisory,
        module: module.to_string(),
        symbol: format!("{}.{}", class.name, property.name),
        kind: SymbolKind::Property,
        confidence: property_confidence(usage, &property.name),
        file: file.path.clone(),
        line: property.line,
        reason: String::new(),
    }
}

fn property_confidence(usage: &ProjectUsage, property: &str) -> Confidence {
    if usage.has_unresolved_member_reference(property) {
        Confidence::Low
    } else {
        Confidence::High
    }
}

fn property_is_live(usage: &ProjectUsage, class: &ClassUnit, property: &str) -> bool {
    class
        .method_attr_use
        .values()
        .any(|attrs| attrs.contains(property))
        || usage.typed_member_is_used(&class.name, property)
        || (usage.property_name_is_unique(property)
            && usage.has_unresolved_member_reference(property))
}

fn class_manages_all_fields(
    file: &ParsedFile,
    profile: &dyn DeadCodeProfile,
    class: &ClassUnit,
) -> bool {
    profile.manages_class_properties(class, file.walked.symbols.decorators.get(&class.name))
}

#[cfg(test)]
#[path = "properties_tests.rs"]
mod tests;
