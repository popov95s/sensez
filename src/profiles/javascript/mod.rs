//! JavaScript language profile. TypeScript reuses these helpers with a
//! different grammar.

pub(crate) mod classunit;
pub(crate) mod conditionals;
pub(crate) mod deadcode;
pub(crate) mod imports;
pub(crate) mod lexeme;
pub(crate) mod obsession;
pub(crate) mod performance;
pub(crate) mod resolve;
pub(crate) mod roots;
pub(crate) mod scope;
pub(crate) mod symbols;
pub(crate) mod tokens;
pub(crate) mod traversal;
pub(crate) mod typehints;
pub(crate) mod typevocab;
pub(crate) mod units;

#[cfg(test)]
mod tests;

use crate::profiles::profile_macro::lang_profile;
use crate::profiles::Language;

lang_profile! {
    /// The JavaScript language profile (zero-sized).
    pub struct JsProfile {
        info: INFO,
        language: Language::JavaScript,
        extensions: &["js", "jsx", "mjs", "cjs"],
        grammar: tree_sitter_javascript::LANGUAGE,
        walk: traversal::walk,
        root_for: roots::root_for,
        module_name: roots::module_name,
        is_package_index: roots::is_package_index,
        containing_package: resolve::containing_package,
        // `./foo` resolves against the importing file's directory on disk.
        resolve_target: |import, _pkg, file, root| resolve::resolve_target(import, file, root),
        // JS named imports are symbols, never submodules.
        submodule_candidate: |_target, _symbol| None,
        decorators: false,
        classify_decorator: deadcode::classify,
        is_conventionally_private: deadcode::is_conventionally_private,
        is_entry_file_stem: deadcode::is_entry_file_stem,
        dead_code_defaults: deadcode::defaults,
        // package.json bin/main derivation: deferred milestone.
        entry_modules: |_root| Vec::new(),
        is_containment: |_importer, _target| false,
    }
}
