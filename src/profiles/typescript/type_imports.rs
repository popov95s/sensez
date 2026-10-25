//! Emit mode of inline `type`-only imports, from `tsconfig` compiler options.

use super::compiler_options::CompilerOptions;
use serde_json::Value;
use std::path::Path;

/// Whether TypeScript emit keeps an import whose specifiers are all inline `type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TypeOnlyImports {
    Erased,
    Kept,
}

pub(super) fn load(config: &Path) -> TypeOnlyImports {
    let options = CompilerOptions::load(config).values;
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
