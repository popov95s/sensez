//! Python generated/data source detection.

use std::path::Path;

const HEADER_SCAN_LINES: usize = 6;

struct PathRule {
    extension: Option<&'static str>,
    file_name: Option<&'static str>,
    parent: Option<&'static str>,
    any_components: &'static [&'static [&'static str]],
    excluded_stems: &'static [&'static str],
}

struct HeaderRule {
    path: PathRule,
    contains_all: &'static [&'static str],
}

const GENERATED_PATH_RULES: &[PathRule] = &[
    PathRule {
        extension: Some("py"),
        file_name: Some("fastjsonschema_validations.py"),
        parent: None,
        any_components: &[],
        excluded_stems: &[],
    },
    PathRule {
        extension: Some("py"),
        file_name: Some("formats.py"),
        parent: None,
        any_components: &[&["conf", "config"], &["locale", "locales"]],
        excluded_stems: &[],
    },
];

const GENERATED_HEADER_RULES: &[HeaderRule] = &[
    HeaderRule {
        path: PathRule {
            extension: Some("py"),
            file_name: None,
            parent: Some("encodings"),
            any_components: &[],
            excluded_stems: &["__init__", "aliases"],
        },
        contains_all: &["Python Character Mapping Codec", "gencodec.py"],
    },
    HeaderRule {
        path: PathRule {
            extension: Some("py"),
            file_name: None,
            parent: None,
            any_components: &[],
            excluded_stems: &[],
        },
        contains_all: &["generated from", "OpenAPI spec", "Stainless"],
    },
];

pub(super) fn is_generated_or_data_source(path: &Path) -> bool {
    GENERATED_PATH_RULES.iter().any(|rule| rule.matches(path))
        || GENERATED_HEADER_RULES.iter().any(|rule| rule.matches(path))
}

impl PathRule {
    fn matches(&self, path: &Path) -> bool {
        self.extension
            .is_none_or(|ext| path.extension().and_then(|s| s.to_str()) == Some(ext))
            && self
                .file_name
                .is_none_or(|name| path.file_name().and_then(|s| s.to_str()) == Some(name))
            && self.parent.is_none_or(|parent| {
                path.parent()
                    .and_then(|p| p.file_name())
                    .and_then(|s| s.to_str())
                    == Some(parent)
            })
            && self.excluded_stems.iter().all(|stem| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .is_none_or(|actual| actual != *stem)
            })
            && self.any_components.iter().all(|choices| {
                path.components()
                    .filter_map(|c| c.as_os_str().to_str())
                    .any(|part| choices.contains(&part))
            })
    }
}

impl HeaderRule {
    fn matches(&self, path: &Path) -> bool {
        if !self.path.matches(path) {
            return false;
        }
        let Ok(text) = std::fs::read_to_string(path) else {
            return false;
        };
        let prefix = text
            .lines()
            .take(HEADER_SCAN_LINES)
            .collect::<Vec<_>>()
            .join("\n");
        self.contains_all
            .iter()
            .all(|needle| prefix.contains(needle))
    }
}

#[cfg(test)]
#[path = "generated_tests.rs"]
mod tests;
