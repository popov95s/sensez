//! Best-effort generated/data source detection.
//!
//! These files are valid source, but they are not useful maintainability
//! targets: generated codec tables, schema validators, and locale data modules
//! dominate duplication/complexity reports while not being code humans should
//! refactor by hand.

use std::path::Path;

const HEADER_SCAN_LINES: usize = 6;

struct PathRule {
    name: &'static str,
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
        name: "fastjsonschema generated validator",
        extension: Some("py"),
        file_name: Some("fastjsonschema_validations.py"),
        parent: None,
        any_components: &[],
        excluded_stems: &[],
    },
    PathRule {
        name: "locale format data module",
        extension: Some("py"),
        file_name: Some("formats.py"),
        parent: None,
        any_components: &[&["conf", "config"], &["locale", "locales"]],
        excluded_stems: &[],
    },
];

const GENERATED_HEADER_RULES: &[HeaderRule] = &[HeaderRule {
    path: PathRule {
        name: "Python generated character-map codec",
        extension: Some("py"),
        file_name: None,
        parent: Some("encodings"),
        any_components: &[],
        excluded_stems: &["__init__", "aliases"],
    },
    contains_all: &["Python Character Mapping Codec", "gencodec.py"],
}];

pub fn is_generated_or_data_source(path: &Path) -> bool {
    GENERATED_PATH_RULES.iter().any(|rule| rule.matches(path))
        || GENERATED_HEADER_RULES.iter().any(|rule| rule.matches(path))
}

impl PathRule {
    fn matches(&self, path: &Path) -> bool {
        let _rule_name = self.name;
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
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn detects_generated_codecs_by_header_not_name_alone() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("encodings");
        fs::create_dir_all(&dir).unwrap();
        let codec = dir.join("cp037.py");
        fs::write(
            &codec,
            "\"\"\" Python Character Mapping Codec cp037 generated from 'x' with gencodec.py.\n\"\"\"\n",
        )
        .unwrap();
        let handwritten = dir.join("custom.py");
        fs::write(&handwritten, "def encode(x):\n    return x\n").unwrap();

        assert!(is_generated_or_data_source(&codec));
        assert!(!is_generated_or_data_source(&handwritten));
    }

    #[test]
    fn detects_schema_validators_and_locale_format_data() {
        assert!(is_generated_or_data_source(Path::new(
            "setuptools/config/_validate_pyproject/fastjsonschema_validations.py"
        )));
        assert!(is_generated_or_data_source(Path::new(
            "django/conf/locale/ar/formats.py"
        )));
        assert!(!is_generated_or_data_source(Path::new(
            "app/locale/formats.py"
        )));
    }

    #[test]
    fn path_rules_are_composable() {
        let rule = PathRule {
            name: "test rule",
            extension: Some("py"),
            file_name: Some("data.py"),
            parent: None,
            any_components: &[&["generated", "autogen"], &["schemas"]],
            excluded_stems: &[],
        };

        assert!(rule.matches(Path::new("pkg/generated/schemas/data.py")));
        assert!(rule.matches(Path::new("pkg/autogen/schemas/data.py")));
        assert!(!rule.matches(Path::new("pkg/generated/other/data.py")));
    }
}
