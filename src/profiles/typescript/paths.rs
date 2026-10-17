//! TypeScript `tsconfig` path-alias resolution.
//!
//! This profile module intentionally owns the TypeScript-specific JSONC and
//! `compilerOptions.paths` behavior. The shared graph receives only the
//! resulting, language-neutral module target.

use super::type_imports::{self, TypeOnlyImports};
use crate::profiles::javascript::{resolve, roots};
use crate::spine::ir::ImportContext;
use serde_json::Value;
use std::collections::hash_map::Entry as CacheEntry;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// TypeScript resolver state scoped to one graph build.
///
/// A scan can include multiple package roots, each with its own alias table.
/// Both valid and empty tables are retained so a missing or malformed config is
/// attempted once per root, rather than once per import.
#[derive(Default)]
pub(crate) struct ResolutionCache {
    aliases_by_root: HashMap<PathBuf, Paths>,
    type_only_imports_by_dir: HashMap<PathBuf, TypeOnlyImports>,
    #[cfg(test)]
    loads: usize,
}

impl ResolutionCache {
    pub(super) fn aliases(&mut self, root: &Path) -> &Paths {
        match self.aliases_by_root.entry(root.to_path_buf()) {
            CacheEntry::Occupied(entry) => entry.into_mut(),
            CacheEntry::Vacant(entry) => {
                #[cfg(test)]
                {
                    self.loads += 1;
                }
                entry.insert(load_aliases(root))
            }
        }
    }

    /// Filesystem path a `tsconfig` alias maps `specifier` to, for modules under `root`.
    pub(crate) fn alias_path(&mut self, root: &Path, specifier: &str) -> Option<PathBuf> {
        self.aliases(root).resolve_path(specifier)
    }

    /// Emit mode of inline type-only imports under the nearest `tsconfig.json` above `dir`.
    pub(super) fn type_only_imports(&mut self, dir: &Path) -> TypeOnlyImports {
        if let Some(&mode) = self.type_only_imports_by_dir.get(dir) {
            return mode;
        }
        let config = dir.join("tsconfig.json");
        let mode = if config.is_file() {
            type_imports::load(&config)
        } else if let Some(parent) = dir.parent() {
            self.type_only_imports(parent)
        } else {
            TypeOnlyImports::Erased
        };
        self.type_only_imports_by_dir
            .insert(dir.to_path_buf(), mode);
        mode
    }

    #[cfg(test)]
    pub(super) fn load_count(&self) -> usize {
        self.loads
    }
}

pub(super) fn resolve_target(
    cache: &mut ResolutionCache,
    import: &ImportContext,
    file: &Path,
    root: &Path,
) -> String {
    cache
        .aliases(root)
        .resolve(&import.target_module)
        .unwrap_or_else(|| resolve::resolve_target(import, file, root))
}

pub(super) struct Paths {
    root: PathBuf,
    base_url: PathBuf,
    entries: Vec<Entry>,
}

struct Entry {
    pattern: String,
    targets: Vec<String>,
}

impl Paths {
    pub(super) fn resolve(&self, specifier: &str) -> Option<String> {
        self.resolve_path(specifier)
            .map(|path| roots::module_name(&path, &self.root))
    }

    fn resolve_path(&self, specifier: &str) -> Option<PathBuf> {
        self.entries
            .iter()
            .filter_map(|entry| match_pattern(&entry.pattern, specifier).map(|part| (entry, part)))
            .max_by_key(|(entry, _)| entry.pattern.len())
            .and_then(|(entry, part)| entry.targets.first().map(|target| (target, part)))
            .map(|(target, part)| self.base_url.join(target.replace('*', part)))
    }
}

/// Parsed JSONC contents of a `tsconfig.json`.
pub(super) fn read_tsconfig(path: &Path) -> Option<Value> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&normalize_jsonc(&text)).ok()
}

fn load_aliases(root: &Path) -> Paths {
    let Some(value) = read_tsconfig(&root.join("tsconfig.json")) else {
        return Paths {
            root: root.to_path_buf(),
            base_url: root.to_path_buf(),
            entries: Vec::new(),
        };
    };
    let options = value.get("compilerOptions").and_then(Value::as_object);
    let base_url = options
        .and_then(|options| options.get("baseUrl"))
        .and_then(Value::as_str)
        .map(|base| root.join(base))
        .unwrap_or_else(|| root.to_path_buf());
    let entries = options
        .and_then(|options| options.get("paths"))
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|paths| paths.iter())
        .filter_map(|(pattern, targets)| {
            let targets: Vec<_> = targets
                .as_array()?
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect();
            (!targets.is_empty()).then(|| Entry {
                pattern: pattern.clone(),
                targets,
            })
        })
        .collect();
    Paths {
        root: root.to_path_buf(),
        base_url,
        entries,
    }
}

fn match_pattern<'a>(pattern: &'a str, specifier: &'a str) -> Option<&'a str> {
    match pattern.split_once('*') {
        Some((prefix, suffix)) if specifier.starts_with(prefix) && specifier.ends_with(suffix) => {
            let end = specifier.len().checked_sub(suffix.len())?;
            Some(&specifier[prefix.len()..end])
        }
        Some(_) => None,
        None if pattern == specifier => Some(""),
        None => None,
    }
}

fn normalize_jsonc(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut state = JsoncState::default();
    while let Some(ch) = chars.next() {
        if state.push_string_char(ch, &mut output) {
            continue;
        }
        if ch == '"' {
            state.start_string();
            output.push(ch);
            continue;
        }
        if ch == '/' && chars.peek() == Some(&'/') {
            chars.next();
            for next in chars.by_ref() {
                if next == '\n' {
                    output.push('\n');
                    break;
                }
            }
            continue;
        }
        if ch == '/' && chars.peek() == Some(&'*') {
            chars.next();
            while let Some(next) = chars.next() {
                if next == '*' && chars.peek() == Some(&'/') {
                    chars.next();
                    break;
                }
                if next == '\n' {
                    output.push('\n');
                }
            }
            continue;
        }
        if ch == ',' {
            let keep_before_comma = output.len();
            output.push(ch);
            while let Some(next) = chars.next_if(|next| next.is_whitespace()) {
                output.push(next);
            }
            if matches!(chars.peek(), Some('}' | ']')) {
                output.truncate(keep_before_comma);
            }
            continue;
        }
        output.push(ch);
    }
    output
}

#[derive(Default)]
struct JsoncState {
    in_string: bool,
    escaped: bool,
}

impl JsoncState {
    fn start_string(&mut self) {
        self.in_string = true;
        self.escaped = false;
    }

    fn push_string_char(&mut self, ch: char, output: &mut String) -> bool {
        if !self.in_string {
            return false;
        }
        let escaped = self.escaped;
        self.escaped = ch == '\\' && !escaped;
        if ch == '"' && !escaped {
            self.in_string = false;
        }
        output.push(ch);
        true
    }
}

#[cfg(test)]
mod normalize_tests {
    use super::normalize_jsonc;

    #[test]
    fn preserves_escaped_quotes_and_comment_markers_in_strings() {
        let input = r#"{ "label": "escaped \" // text", "trailing": 1, }"#;
        let normalized = normalize_jsonc(input);
        let value: serde_json::Value = serde_json::from_str(&normalized).unwrap();
        assert_eq!(value["label"], "escaped \" // text");
        assert_eq!(value["trailing"], 1);
    }

    #[test]
    fn strips_comments_and_trailing_commas_into_parseable_json() {
        let input = "{ /* c */ \"a\": [1, 2,], // line\n \"b\": { \"c\": 1, }, }";
        let normalized = normalize_jsonc(input);
        let value: serde_json::Value = serde_json::from_str(&normalized).unwrap_or_else(|err| {
            panic!("normalized output must be valid JSON: {err}\n{normalized}")
        });
        assert_eq!(value["a"], serde_json::json!([1, 2]));
        assert_eq!(value["b"]["c"], 1);
        assert!(!normalized.contains("//"), "line comments must be stripped");
    }
}
