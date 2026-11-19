//! Class-shape duplication: same names and overlapping typed properties.

use crate::report::{ActionLevel, CloneClass, CloneDetector, CloneOccurrence};
use crate::spine::ir::Language;
use crate::spine::parser::{ClassProperty, ParsedFile};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

pub fn detect(
    files: &[&ParsedFile],
    class_name_duplicates: bool,
    min_overlap: usize,
) -> Vec<CloneClass> {
    let classes = collect_classes(files);
    let mut out = if class_name_duplicates {
        same_name_findings(&classes)
    } else {
        Vec::new()
    };
    if min_overlap > 0 {
        out.extend(property_overlap_findings(&classes, min_overlap));
    }
    out
}

struct ClassShape {
    family: Language,
    name: String,
    occurrence: CloneOccurrence,
    properties: BTreeMap<PropertyKey, usize>,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct PropertyKey {
    name: String,
    type_name: String,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct PairKey {
    left_file: PathBuf,
    left_row: usize,
    right_file: PathBuf,
    right_row: usize,
}

fn collect_classes(files: &[&ParsedFile]) -> Vec<ClassShape> {
    let mut out = Vec::new();
    for file in files {
        for class in &file.walked.units.classes {
            if class.name.is_empty() {
                continue;
            }
            out.push(ClassShape {
                family: crate::profiles::registry::clone_family(file.language),
                name: class.name.clone(),
                occurrence: CloneOccurrence {
                    file: file.path.clone(),
                    start_row: class.start_line,
                    end_row: class.end_line,
                },
                properties: property_map(&class.properties),
            });
        }
    }
    out
}

fn property_map(properties: &[ClassProperty]) -> BTreeMap<PropertyKey, usize> {
    let mut out = BTreeMap::new();
    for property in properties {
        if property.name.is_empty() || property.type_name.is_empty() {
            continue;
        }
        out.entry(PropertyKey {
            name: property.name.clone(),
            type_name: property.type_name.clone(),
        })
        .or_insert(property.line);
    }
    out
}

fn same_name_findings(classes: &[ClassShape]) -> Vec<CloneClass> {
    let mut groups: BTreeMap<(Language, &str), Vec<&ClassShape>> = BTreeMap::new();
    for class in classes {
        groups
            .entry((class.family, class.name.as_str()))
            .or_default()
            .push(class);
    }
    groups
        .into_values()
        .filter(|group| distinct_locations(group) >= 2)
        .map(|group| CloneClass {
            action: ActionLevel::Info,
            token_length: 0,
            occurrences: group.iter().map(|class| class.occurrence.clone()).collect(),
            hint: Some(format!(
                "class name `{}` appears in multiple places",
                group[0].name
            )),
            detector: CloneDetector::ClassShape,
            absorbed: Vec::new(),
        })
        .collect()
}

fn property_overlap_findings(classes: &[ClassShape], min_overlap: usize) -> Vec<CloneClass> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for (i, left) in classes.iter().enumerate() {
        if left.properties.len() < min_overlap {
            continue;
        }
        for right in classes.iter().skip(i + 1) {
            if left.family != right.family || right.properties.len() < min_overlap {
                continue;
            }
            let shared = shared_properties(left, right);
            if shared.len() < min_overlap || !seen.insert(pair_key(left, right)) {
                continue;
            }
            out.push(CloneClass {
                action: ActionLevel::Advisory,
                token_length: shared.len(),
                occurrences: vec![left.occurrence.clone(), right.occurrence.clone()],
                hint: Some(format!(
                    "class property overlap: {} shared typed properties ({})",
                    shared.len(),
                    shared.join(", ")
                )),
                detector: CloneDetector::ClassShape,
                absorbed: Vec::new(),
            });
        }
    }
    out
}

fn shared_properties(left: &ClassShape, right: &ClassShape) -> Vec<String> {
    left.properties
        .keys()
        .filter(|key| right.properties.contains_key(*key))
        .map(|key| format!("{}: {}", key.name, key.type_name))
        .collect()
}

fn distinct_locations(group: &[&ClassShape]) -> usize {
    group
        .iter()
        .map(|class| (&class.occurrence.file, class.occurrence.start_row))
        .collect::<BTreeSet<_>>()
        .len()
}

fn pair_key(left: &ClassShape, right: &ClassShape) -> PairKey {
    let a = (left.occurrence.file.clone(), left.occurrence.start_row);
    let b = (right.occurrence.file.clone(), right.occurrence.start_row);
    if a <= b {
        PairKey {
            left_file: a.0,
            left_row: a.1,
            right_file: b.0,
            right_row: b.1,
        }
    } else {
        PairKey {
            left_file: b.0,
            left_row: b.1,
            right_file: a.0,
            right_row: a.1,
        }
    }
}

#[cfg(test)]
#[path = "class_shapes_tests.rs"]
mod tests;
