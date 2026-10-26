//! Emit mode of inline `type`-only imports, from `tsconfig` compiler options.

use super::tsconfig::read_tsconfig;
use serde_json::{Map, Value};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

const MAX_EXTENDS_DEPTH: usize = 16;

/// Whether TypeScript emit keeps an import whose specifiers are all inline `type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TypeOnlyImports {
    Erased,
    Kept,
}

pub(super) fn load(config: &Path) -> TypeOnlyImports {
    let options = compiler_options(config, 0);
    let flag = |key: &str| options.get(key).and_then(Value::as_bool) == Some(true);
    let kept_as_values = matches!(
        options
            .get("importsNotUsedAsValues")
            .and_then(Value::as_str),
        Some("preserve" | "error")
    );
    if flag("verbatimModuleSyntax") || flag("preserveValueImports") || kept_as_values {
        TypeOnlyImports::Kept
    } else {
        TypeOnlyImports::Erased
    }
}

/// `compilerOptions` of `config` merged over its relative `extends` chain.
fn compiler_options(config: &Path, depth: usize) -> Map<String, Value> {
    let Some(value) = read_tsconfig(config) else {
        return Map::new();
    };
    let dir = config.parent().unwrap_or(Path::new(""));
    let mut options = Map::new();
    if depth < MAX_EXTENDS_DEPTH {
        for base in extends(&value) {
            options.extend(compiler_options(&base_config(dir, base), depth + 1));
        }
    }
    if let Some(own) = value.get("compilerOptions").and_then(Value::as_object) {
        options.extend(own.clone());
    }
    options
}

/// Relative `extends` entries; package bases fall back to TypeScript defaults.
fn extends(value: &Value) -> Vec<&str> {
    let entries = match value.get("extends") {
        Some(Value::String(base)) => vec![base.as_str()],
        Some(Value::Array(bases)) => bases.iter().filter_map(Value::as_str).collect(),
        _ => Vec::new(),
    };
    entries
        .into_iter()
        .filter(|base| base.starts_with('.'))
        .collect()
}

fn base_config(dir: &Path, base: &str) -> PathBuf {
    let path = dir.join(base);
    if path.is_file() {
        return path;
    }
    let mut with_json = OsString::from(path);
    with_json.push(".json");
    PathBuf::from(with_json)
}
