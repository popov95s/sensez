use crate::profiles::registry;
use crate::spine::ir::TypeHints;
use crate::spine::parser::ParsedFile;
use std::collections::{HashMap, HashSet};

use super::exports::exposed_classes;
use super::overrides::override_methods;
use super::properties_index::{project_class_index, type_parts, ClassIndex};

#[derive(Default)]
pub(super) struct ProjectUsage {
    typed_member_uses: HashSet<(String, String)>,
    returned_member_uses: HashSet<(String, String)>,
    untyped_attrs: HashSet<String>,
    chained_attrs: HashSet<String>,
    string_literals: HashSet<String>,
    property_name_counts: HashMap<String, usize>,
    method_name_counts: HashMap<String, usize>,
    classes_with_member_evidence: HashSet<String>,
    override_methods: HashSet<(String, String)>,
    exposed_classes: HashSet<String>,
}

impl ProjectUsage {
    pub(super) fn from_files(files: &[&ParsedFile]) -> Self {
        let class_index = project_class_index(files);
        let (property_name_counts, method_name_counts) = member_name_counts(files);
        let mut usage = Self {
            property_name_counts,
            method_name_counts,
            classes_with_member_evidence: class_member_evidence(files),
            override_methods: override_methods(files, &class_index),
            exposed_classes: exposed_classes(files, &class_index),
            ..Self::default()
        };
        files
            .iter()
            .map(|file| FileUsage::from_file(file, &class_index))
            .for_each(|file_usage| usage.merge(file_usage));
        usage
    }

    pub(super) fn class_has_member_evidence(&self, class_name: &str) -> bool {
        self.classes_with_member_evidence.contains(class_name)
    }

    pub(super) fn property_name_is_unique(&self, property: &str) -> bool {
        self.property_name_counts.get(property) == Some(&1)
    }

    pub(super) fn method_name_is_unique(&self, method: &str) -> bool {
        self.method_name_counts.get(method) == Some(&1)
    }

    pub(super) fn method_overrides_base(&self, class: &str, method: &str) -> bool {
        self.override_methods
            .contains(&(class.to_string(), method.to_string()))
    }

    pub(super) fn class_is_exposed_api(&self, class: &str) -> bool {
        self.exposed_classes.contains(class)
    }

    pub(super) fn typed_member_is_used(&self, class: &str, member: &str) -> bool {
        let key = (class.to_string(), member.to_string());
        self.typed_member_uses.contains(&key) || self.returned_member_uses.contains(&key)
    }

    pub(super) fn has_unresolved_member_reference(&self, member: &str) -> bool {
        self.untyped_attrs.contains(member)
            || self.chained_attrs.contains(member)
            || self.string_literals.contains(member)
    }

    fn merge(&mut self, file_usage: FileUsage) {
        self.typed_member_uses.extend(file_usage.typed_member_uses);
        self.returned_member_uses
            .extend(file_usage.returned_member_uses);
        self.untyped_attrs.extend(file_usage.untyped_attrs);
        self.chained_attrs.extend(file_usage.chained_attrs);
        self.string_literals.extend(file_usage.string_literals);
        self.classes_with_member_evidence
            .extend(file_usage.classes_with_member_evidence);
    }
}

#[derive(Default)]
struct FileUsage {
    typed_member_uses: HashSet<(String, String)>,
    returned_member_uses: HashSet<(String, String)>,
    untyped_attrs: HashSet<String>,
    chained_attrs: HashSet<String>,
    string_literals: HashSet<String>,
    classes_with_member_evidence: HashSet<String>,
}

impl FileUsage {
    fn from_file(file: &ParsedFile, class_index: &ClassIndex) -> Self {
        let receivers = typed_receivers(file, class_index);
        let (typed_member_uses, typed_classes) = typed_member_evidence(
            &file.walked.usage.attribute_accesses,
            &receivers,
            class_index,
        );
        let (path_member_uses, path_classes) = typed_member_evidence(
            &file.walked.usage.attribute_path_accesses,
            &receivers,
            class_index,
        );
        let (returned_member_uses, returned_classes) = returned_member_evidence(file, class_index);
        let unresolved = unresolved_member_evidence(
            &file.walked.usage.attribute_accesses,
            &file.walked.usage.chained_attribute_names,
            &file.walked.usage.string_literals,
            &receivers,
        );

        Self {
            typed_member_uses: typed_member_uses
                .into_iter()
                .chain(path_member_uses)
                .collect(),
            returned_member_uses,
            untyped_attrs: unresolved.untyped_attrs,
            chained_attrs: unresolved.chained_attrs,
            string_literals: unresolved.string_literals,
            classes_with_member_evidence: typed_classes
                .into_iter()
                .chain(path_classes)
                .chain(returned_classes)
                .collect(),
        }
    }
}

pub(super) fn typed_receivers(
    file: &ParsedFile,
    class_index: &ClassIndex,
) -> HashMap<String, String> {
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

fn typed_member_evidence(
    accesses: &HashMap<String, HashSet<String>>,
    receivers: &HashMap<String, String>,
    class_index: &ClassIndex,
) -> (HashSet<(String, String)>, HashSet<String>) {
    let classes: HashSet<String> = accesses
        .keys()
        .filter_map(|base| receivers.get(base))
        .cloned()
        .collect();
    let uses = accesses
        .iter()
        .filter_map(|(base, attrs)| receivers.get(base).map(|class| (class.as_str(), attrs)))
        .flat_map(|(class, attrs)| member_uses_for_class(class, attrs, class_index))
        .collect();

    (uses, classes)
}

fn returned_member_evidence(
    file: &ParsedFile,
    class_index: &ClassIndex,
) -> (HashSet<(String, String)>, HashSet<String>) {
    file.walked
        .usage
        .call_result_attribute_accesses
        .iter()
        .filter_map(|(function, attrs)| {
            type_hints(file)
                .return_types
                .get(function)
                .and_then(|ty| type_class(ty, class_index))
                .map(|class| (class, attrs))
        })
        .fold(
            (HashSet::new(), HashSet::new()),
            |mut out, (class, attrs)| {
                out.1.insert(class.to_string());
                out.0
                    .extend(member_uses_for_class(class, attrs, class_index));
                out
            },
        )
}

fn member_uses_for_class(
    class: &str,
    attrs: &HashSet<String>,
    class_index: &ClassIndex,
) -> Vec<(String, String)> {
    class_index
        .class_and_descendants(class)
        .into_iter()
        .flat_map(|target| attrs.iter().map(move |attr| (target.clone(), attr.clone())))
        .collect()
}

fn untyped_attribute_names(
    attribute_accesses: &HashMap<String, HashSet<String>>,
    receivers: &HashMap<String, String>,
) -> HashSet<String> {
    attribute_accesses
        .iter()
        .filter(|(base, _)| !receivers.contains_key(base.as_str()))
        .flat_map(|(_, attrs)| attrs.iter().cloned())
        .collect()
}

#[derive(Default)]
struct UnresolvedMemberEvidence {
    untyped_attrs: HashSet<String>,
    chained_attrs: HashSet<String>,
    string_literals: HashSet<String>,
}

fn unresolved_member_evidence(
    attribute_accesses: &HashMap<String, HashSet<String>>,
    chained_attribute_names: &HashSet<String>,
    string_literals: &HashSet<String>,
    receivers: &HashMap<String, String>,
) -> UnresolvedMemberEvidence {
    UnresolvedMemberEvidence {
        untyped_attrs: untyped_attribute_names(attribute_accesses, receivers),
        chained_attrs: chained_attribute_names.iter().cloned().collect(),
        string_literals: string_literals.iter().cloned().collect(),
    }
}

fn class_member_evidence(files: &[&ParsedFile]) -> HashSet<String> {
    files
        .iter()
        .flat_map(|file| file.walked.units.classes.iter())
        .filter(|class| {
            class
                .method_attr_use
                .values()
                .any(|attrs| !attrs.is_empty())
        })
        .map(|class| class.name.clone())
        .collect()
}

fn member_name_counts(files: &[&ParsedFile]) -> (HashMap<String, usize>, HashMap<String, usize>) {
    let mut property_counts = HashMap::new();
    let mut method_counts = HashMap::new();
    files.iter().for_each(|file| {
        let profile = registry::dead_code_profile(file.language);
        file.walked.units.classes.iter().for_each(|class| {
            if profile
                .manages_class_properties(class, file.walked.symbols.decorators.get(&class.name))
            {
                return;
            }
            class
                .properties
                .iter()
                .filter(|property| {
                    !property.name.starts_with('_') && !profile.manages_property(property)
                })
                .for_each(|property| {
                    *property_counts.entry(property.name.clone()).or_insert(0) += 1;
                });
            class.methods.iter().for_each(|method| {
                *method_counts.entry(method.clone()).or_insert(0) += 1;
            });
        });
    });
    (property_counts, method_counts)
}
