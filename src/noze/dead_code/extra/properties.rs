//! Opt-in dead-property detection.
//!
//! This pass is class-aware: a property is live when its owning class methods
//! use it, when a typed receiver uses it, when the same file has an untyped
//! attribute access to that name, or when the name appears as a string literal
//! for dynamic APIs/serializers.

use crate::profiles::{registry, DeadCodeProfile};
use crate::report::{ActionLevel, Confidence, DeadCodeFinding};
use crate::spine::ir::{ClassProperty, ClassUnit, TypeHints};
use crate::spine::parser::{ParsedFile, SymbolKind};
use properties_index::{project_class_index, type_parts, ClassIndex};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

#[path = "properties_index.rs"]
mod properties_index;

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
        .map(|property| finding(file, module, class, property))
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
            || usage.class_has_property_evidence(&class.name))
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

fn typed_receivers(file: &ParsedFile, class_index: &ClassIndex) -> HashMap<String, String> {
    let hints = type_hints(file);
    let mut out = HashMap::new();
    for (name, ty) in &hints.var_types {
        if let Some(class) = type_class(ty, class_index) {
            out.insert(name.to_string(), class.to_string());
        }
    }
    for (name, ty) in &hints.attr_types {
        if let Some(class) = type_class(ty, class_index) {
            out.insert(name.to_string(), class.to_string());
        }
    }
    for ((_, name), ty) in &hints.param_types {
        if let Some(class) = type_class(ty, class_index) {
            out.insert(name.to_string(), class.to_string());
        }
    }
    out
}

fn type_hints(file: &ParsedFile) -> &TypeHints {
    &file.walked.units.type_hints
}

fn type_class<'a>(ty: &str, class_index: &'a ClassIndex) -> Option<&'a str> {
    type_parts(ty).find_map(|part| class_index.get(part.trim_matches('?')))
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
        || (usage.property_name_is_unique(property)
            && (usage.untyped_attrs.contains(property)
                || usage.chained_attrs.contains(property)
                || usage.string_literals.contains(property)))
}

#[derive(Default)]
struct ProjectUsage {
    typed_property_uses: HashSet<(String, String)>,
    returned_property_uses: HashSet<(String, String)>,
    untyped_attrs: HashSet<String>,
    chained_attrs: HashSet<String>,
    string_literals: HashSet<String>,
    property_name_counts: HashMap<String, usize>,
    classes_with_property_evidence: HashSet<String>,
}

impl ProjectUsage {
    fn from_files(files: &[&ParsedFile]) -> Self {
        let class_index = project_class_index(files);
        let mut usage = Self {
            property_name_counts: property_name_counts(files),
            ..Self::default()
        };
        for file in files {
            usage.collect_method_property_evidence(file);
            let receivers = typed_receivers(file, &class_index);
            usage
                .untyped_attrs
                .extend(untyped_attribute_names(file, &receivers));
            usage
                .chained_attrs
                .extend(file.walked.usage.chained_attribute_names.iter().cloned());
            usage
                .string_literals
                .extend(file.walked.usage.string_literals.iter().cloned());
            usage.collect_typed_property_uses(
                &file.walked.usage.attribute_accesses,
                &receivers,
                &class_index,
            );
            usage.collect_typed_property_uses(
                &file.walked.usage.attribute_path_accesses,
                &receivers,
                &class_index,
            );
            usage.collect_returned_property_uses(file, &class_index);
        }
        usage
    }

    fn property_name_is_unique(&self, property: &str) -> bool {
        self.property_name_counts.get(property) == Some(&1)
    }

    fn class_has_property_evidence(&self, class_name: &str) -> bool {
        self.classes_with_property_evidence.contains(class_name)
    }

    fn collect_method_property_evidence(&mut self, file: &ParsedFile) {
        for class in &file.walked.units.classes {
            if class
                .method_attr_use
                .values()
                .any(|attrs| !attrs.is_empty())
            {
                self.classes_with_property_evidence
                    .insert(class.name.clone());
            }
        }
    }

    fn collect_typed_property_uses(
        &mut self,
        accesses: &HashMap<String, HashSet<String>>,
        receivers: &HashMap<String, String>,
        class_index: &ClassIndex,
    ) {
        for (base, attrs) in accesses {
            if let Some(class) = receivers.get(base) {
                self.classes_with_property_evidence.insert(class.clone());
                for target in class_index.class_and_descendants(class) {
                    self.typed_property_uses
                        .extend(attrs.iter().map(|attr| (target.clone(), attr.clone())));
                }
            }
        }
    }

    fn collect_returned_property_uses(&mut self, file: &ParsedFile, class_index: &ClassIndex) {
        for (function, attrs) in &file.walked.usage.call_result_attribute_accesses {
            if let Some(ty) = type_hints(file).return_types.get(function) {
                if let Some(class) = type_class(ty, class_index) {
                    self.classes_with_property_evidence
                        .insert(class.to_string());
                    for target in class_index.class_and_descendants(class) {
                        self.returned_property_uses
                            .extend(attrs.iter().map(|attr| (target.clone(), attr.clone())));
                    }
                }
            }
        }
    }
}

fn property_name_counts(files: &[&ParsedFile]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for file in files {
        let profile = registry::dead_code_profile(file.language);
        for class in reportable_classes(file) {
            if class_manages_all_fields(file, profile, class) {
                continue;
            }
            for property in &class.properties {
                if !property.name.starts_with('_') && !profile.manages_property(property) {
                    *counts.entry(property.name.clone()).or_insert(0) += 1;
                }
            }
        }
    }
    counts
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
