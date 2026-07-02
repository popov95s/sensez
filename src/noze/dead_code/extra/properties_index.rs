use crate::spine::ir::ClassUnit;
use crate::spine::parser::ParsedFile;
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub(super) struct ClassIndex {
    names: HashMap<String, String>,
    bases: HashMap<String, Vec<String>>,
    children: HashMap<String, Vec<String>>,
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
        let Some(children) = self.children.get(class) else {
            return;
        };
        for candidate in children {
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
    index.children = child_index(&index);
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

fn child_index(index: &ClassIndex) -> HashMap<String, Vec<String>> {
    let mut children: HashMap<String, Vec<String>> = HashMap::new();
    for (class, bases) in &index.bases {
        for base in bases {
            for part in type_parts(base) {
                if index.names.contains_key(part) {
                    children
                        .entry(part.to_string())
                        .or_default()
                        .push(class.clone());
                }
            }
        }
    }
    children
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
