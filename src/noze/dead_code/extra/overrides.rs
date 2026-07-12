use super::properties_index::ClassIndex;
use crate::spine::parser::ParsedFile;
use std::collections::{HashMap, HashSet};

pub(super) fn override_methods(
    files: &[&ParsedFile],
    class_index: &ClassIndex,
) -> HashSet<(String, String)> {
    let methods_by_class = methods_by_class(files);
    methods_by_class
        .iter()
        .flat_map(|(class, methods)| {
            methods
                .iter()
                .filter(|method| {
                    overrides_known_base_method(class, method, &methods_by_class, class_index)
                })
                .map(|method| (class.clone(), method.clone()))
        })
        .collect()
}

fn methods_by_class(files: &[&ParsedFile]) -> HashMap<String, HashSet<String>> {
    files
        .iter()
        .flat_map(|file| file.walked.units.classes.iter())
        .map(|class| {
            (
                class.name.clone(),
                class.methods.iter().cloned().collect::<HashSet<_>>(),
            )
        })
        .collect()
}

fn overrides_known_base_method(
    class: &str,
    method: &str,
    methods_by_class: &HashMap<String, HashSet<String>>,
    class_index: &ClassIndex,
) -> bool {
    class_index.ancestors(class).iter().any(|base| {
        methods_by_class
            .get(base)
            .is_some_and(|base_methods| base_methods.contains(method))
    })
}
