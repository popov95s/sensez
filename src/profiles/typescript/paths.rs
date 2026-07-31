//! TypeScript `tsconfig` path-alias resolution.
//!
//! This profile module intentionally owns the TypeScript-specific JSONC and
//! `compilerOptions.paths` behavior. The shared graph receives only the
//! resulting, language-neutral module target.

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
        self.entries
            .iter()
            .filter_map(|entry| match_pattern(&entry.pattern, specifier).map(|part| (entry, part)))
            .max_by_key(|(entry, _)| entry.pattern.len())
            .and_then(|(entry, part)| entry.targets.first().map(|target| (target, part)))
            .map(|(target, part)| self.base_url.join(target.replace('*', part)))
            .map(|path| roots::module_name(&path, &self.root))
    }
}

fn load_aliases(root: &Path) -> Paths {
    let path = root.join("tsconfig.json");
    let Ok(text) = std::fs::read_to_string(path) else {
        return Paths {
            root: root.to_path_buf(),
            base_url: root.to_path_buf(),
            entries: Vec::new(),
        };
    };
    let normalized = normalize_jsonc(&text);
    let Ok(value) = serde_json::from_str::<Value>(&normalized) else {
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
            let mut lookahead = chars.clone();
            while lookahead.peek().is_some_and(|next| next.is_whitespace()) {
                lookahead.next();
            }
            if matches!(lookahead.peek(), Some('}' | ']')) {
                continue;
            }
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
        self.escaped = ch == '\\' && !self.escaped;
        if ch == '"' && !self.escaped {
            self.in_string = false;
        }
        output.push(ch);
        true
    }
}
