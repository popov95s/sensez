use crate::spine::ir::ClassUnit;
use crate::spine::parser::ParsedFile;
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub(super) struct ClassIndex {
    names: HashMap<String, String>,
    bases: HashMap<String, Vec<String>>,
}

impl ClassIndex {
    pub(super) fn get(&self, name: &str) -> Option<&str> {
        self.names.get(name).map(String::as_str)
    }

    pub(super) fn class_and_descendants(&self, class: &str) -> Vec<String> {
        let mut out = vec![class.to_string()];
        let mut seen = HashSet::from([class.to_string()]);
        self.collect_descendants(class, &mut seen, &mut out);
        out
    }

    fn collect_descendants(&self, class: &str, seen: &mut HashSet<String>, out: &mut Vec<String>) {
        for (candidate, bases) in &self.bases {
            if !bases
                .iter()
                .any(|base| type_parts(base).any(|part| part == class))
            {
                continue;
            }
            if seen.insert(candidate.clone()) {
                out.push(candidate.clone());
                self.collect_descendants(candidate, seen, out);
            }
        }
    }
}

pub(super) fn project_class_index(files: &[&ParsedFile]) -> ClassIndex {
    let mut index = ClassIndex::default();
    for class in files
        .iter()
        .flat_map(|file| file.walked.units.classes.iter())
    {
        add_class(&mut index, class);
    }
    index
}

fn add_class(index: &mut ClassIndex, class: &ClassUnit) {
    index
        .names
        .entry(class.name.clone())
        .or_insert_with(|| class.name.clone());
    index
        .bases
        .entry(class.name.clone())
        .or_insert_with(|| class.bases.clone());
}

pub(super) fn type_parts(text: &str) -> impl Iterator<Item = &str> {
    text.split(|ch: char| !(ch == '.' || ch == '_' || ch == '$' || ch.is_alphanumeric()))
        .flat_map(|part| part.rsplit('.'))
        .filter(|part| !part.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_parts_extracts_identifier_segments() {
        let found: Vec<_> = type_parts(
            "typing.Optional['pkg.models.Foo'] | crate::domain::Thing | React.Component<Props>",
        )
        .collect();

        assert_eq!(
            found,
            vec![
                "Optional",
                "typing",
                "Foo",
                "models",
                "pkg",
                "crate",
                "domain",
                "Thing",
                "Component",
                "React",
                "Props"
            ]
        );
    }
}
