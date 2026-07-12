use super::properties_index::ClassIndex;
use crate::profiles::registry;
use crate::spine::parser::{ImportPhase, ParsedFile};
use std::collections::HashSet;

pub(super) fn exposed_classes(files: &[&ParsedFile], class_index: &ClassIndex) -> HashSet<String> {
    let declared_classes = declared_class_names(files);
    let exported = explicitly_exported_class_names(files, &declared_classes);
    exported
        .iter()
        .flat_map(|class| std::iter::once(class.clone()).chain(class_index.ancestors(class)))
        .collect()
}

fn declared_class_names(files: &[&ParsedFile]) -> HashSet<String> {
    files
        .iter()
        .flat_map(|file| file.walked.units.classes.iter())
        .map(|class| class.name.clone())
        .collect()
}

fn explicitly_exported_class_names(
    files: &[&ParsedFile],
    declared_classes: &HashSet<String>,
) -> HashSet<String> {
    files
        .iter()
        .flat_map(|file| {
            dunder_all_class_names(file, declared_classes)
                .into_iter()
                .chain(package_reexported_class_names(file, declared_classes))
        })
        .collect()
}

fn dunder_all_class_names(
    file: &ParsedFile,
    declared_classes: &HashSet<String>,
) -> HashSet<String> {
    file.walked
        .symbols
        .dunder_all
        .iter()
        .flat_map(|names| names.iter())
        .filter(|name| declared_classes.contains(*name))
        .cloned()
        .collect()
}

fn package_reexported_class_names(
    file: &ParsedFile,
    declared_classes: &HashSet<String>,
) -> HashSet<String> {
    if !registry::module_profile(file.language).is_package_index(&file.path) {
        return HashSet::new();
    }
    file.walked
        .symbols
        .imports
        .iter()
        .filter(|import| !import.is_inline && import.phase == ImportPhase::Runtime)
        .flat_map(|import| import.bindings.iter())
        .filter(|binding| is_public_name(binding) && declared_classes.contains(*binding))
        .cloned()
        .collect()
}

fn is_public_name(name: &str) -> bool {
    !name.starts_with('_')
}
