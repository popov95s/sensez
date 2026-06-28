//! Dynamic class entrypoints discovered by configured base-class conventions.
//!
//! Some frameworks instantiate subclasses by config strings, manifests, or
//! reflection rather than ordinary imports. The dead-code pillar only needs the
//! language-neutral rule: a top-level class is live when it inherits from one of
//! the configured dynamic bases, directly or through another class in the file.

use crate::profiles::registry;
use crate::spine::parser::{ParsedFile, SymbolKind};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Default)]
pub(super) struct ClassEntrypoints {
    classes_by_file: HashMap<PathBuf, HashSet<String>>,
}

impl ClassEntrypoints {
    pub(super) fn from_files(files: &[ParsedFile], root_bases: &[String]) -> Self {
        let mut classes_by_file = HashMap::new();
        let configured_bases = root_bases;

        for file in files {
            let defaults = registry::dead_code_profile(file.language).dead_code_defaults();
            if configured_bases.is_empty() && defaults.entrypoint_bases.is_empty() {
                continue;
            }
            let live_bases = merged_bases(configured_bases, defaults.entrypoint_bases);
            let class_bases: HashMap<&str, Vec<String>> = file
                .walked
                .units
                .classes
                .iter()
                .map(|class| (class.name.as_str(), class.bases.clone()))
                .collect();
            let live = dynamic_classes(&class_bases, &live_bases);
            if !live.is_empty() {
                classes_by_file.insert(file.path.clone(), live);
            }
        }

        Self { classes_by_file }
    }

    pub(super) fn is_entrypoint(&self, file: &Path, symbol: &str, kind: SymbolKind) -> bool {
        kind == SymbolKind::Class
            && self
                .classes_by_file
                .get(file)
                .is_some_and(|classes| classes.contains(symbol))
    }
}

fn merged_bases<'a>(
    configured: &'a [String],
    defaults: &'static [&'static str],
) -> HashSet<&'a str> {
    let mut out: HashSet<&'a str> = defaults.iter().copied().collect();
    out.extend(configured.iter().map(String::as_str));
    out
}

fn dynamic_classes(
    class_bases: &HashMap<&str, Vec<String>>,
    root_bases: &HashSet<&str>,
) -> HashSet<String> {
    let mut live = HashSet::new();
    while add_dynamic_classes(class_bases, root_bases, &mut live) {}
    live
}

fn add_dynamic_classes(
    class_bases: &HashMap<&str, Vec<String>>,
    root_bases: &HashSet<&str>,
    live: &mut HashSet<String>,
) -> bool {
    let mut added = false;
    for (class, bases) in class_bases {
        if live.contains(*class) {
            continue;
        }
        if bases
            .iter()
            .any(|base| is_dynamic_base(base, live, root_bases))
        {
            live.insert((*class).to_string());
            added = true;
        }
    }
    added
}

fn is_dynamic_base(base: &str, live: &HashSet<String>, root_bases: &HashSet<&str>) -> bool {
    let short = base.rsplit('.').next().unwrap_or(base);
    live.contains(short) || root_bases.contains(short) || root_bases.contains(base)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spine::parser::parse_file;
    use std::fs;

    #[test]
    fn subclasses_of_configured_bases_are_entrypoints() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("apps.py");
        fs::write(
            &file,
            "from framework import AppConfig\n\n\
             class SimpleAdminConfig(AppConfig):\n    pass\n\n\
             class AdminConfig(SimpleAdminConfig):\n    pass\n\n\
             class Plain:\n    pass\n",
        )
        .unwrap();
        let parsed = parse_file(&file, 0).unwrap();
        let entrypoints = ClassEntrypoints::from_files(&[parsed], &["AppConfig".to_string()]);

        assert!(entrypoints.is_entrypoint(&file, "AdminConfig", SymbolKind::Class));
        assert!(entrypoints.is_entrypoint(&file, "SimpleAdminConfig", SymbolKind::Class));
        assert!(!entrypoints.is_entrypoint(&file, "Plain", SymbolKind::Class));
    }

    #[test]
    fn dotted_bases_match_by_short_or_full_name() {
        let roots = HashSet::from(["framework.PluginBase"]);
        let mut class_bases = HashMap::new();
        class_bases.insert("Plugin", vec!["framework.PluginBase".to_string()]);

        assert!(dynamic_classes(&class_bases, &roots).contains("Plugin"));

        let roots = HashSet::from(["PluginBase"]);
        assert!(dynamic_classes(&class_bases, &roots).contains("Plugin"));
    }
}
