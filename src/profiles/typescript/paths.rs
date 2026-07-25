//! TypeScript `tsconfig` path-alias resolution.
//!
//! This profile module intentionally owns the TypeScript-specific JSONC and
//! `compilerOptions.paths` behavior. The shared graph receives only the
//! resulting, language-neutral module target.

use crate::profiles::javascript::{resolve, roots};
use crate::spine::ir::ImportContext;
use serde_json::Value;
use std::path::{Path, PathBuf};

pub(super) fn resolve_target(import: &ImportContext, file: &Path, root: &Path) -> String {
    aliases(root)
        .resolve(&import.target_module)
        .unwrap_or_else(|| resolve::resolve_target(import, file, root))
}

struct Paths {
    root: PathBuf,
    base_url: PathBuf,
    entries: Vec<Entry>,
}

struct Entry {
    pattern: String,
    targets: Vec<String>,
}

impl Paths {
    fn resolve(&self, specifier: &str) -> Option<String> {
        self.entries
            .iter()
            .filter_map(|entry| match_pattern(&entry.pattern, specifier).map(|part| (entry, part)))
            .max_by_key(|(entry, _)| entry.pattern.len())
            .and_then(|(entry, part)| entry.targets.first().map(|target| (target, part)))
            .map(|(target, part)| self.base_url.join(target.replace('*', part)))
            .map(|path| roots::module_name(&path, &self.root))
    }
}

fn aliases(root: &Path) -> Paths {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_exact_and_wildcard_paths_from_jsonc() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("tsconfig.json"),
            r#"{ /* comment */ "compilerOptions": { "paths": {
                "@/*": ["src/*"], "@scope/library": ["packages/library/src/index.ts"],
            }, }, }"#,
        )
        .unwrap();
        let paths = aliases(tmp.path());
        assert_eq!(paths.resolve("@/ui/button"), Some("src/ui/button".into()));
        assert_eq!(
            paths.resolve("@scope/library"),
            Some("packages/library/src".into())
        );
    }
}
