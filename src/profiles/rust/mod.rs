//! Scope matches the JS/TS profiles: structural pillars (duplication, cycles,
//! boundaries) plus graph-based dead code for `pub` items (rustc's `dead_code`
//! lint already owns private ones). Per-function smell units are a deferred
//! milestone, as for JS/TS.

pub(crate) mod deadcode;
pub(crate) mod imports;
pub(crate) mod lexeme;
pub(crate) mod performance;
pub(crate) mod resolve;
pub(crate) mod roots;
pub(crate) mod scope;
pub(crate) mod symbols;
pub(crate) mod tokens;
pub(crate) mod traversal;

#[cfg(test)]
mod tests;

use crate::profiles::profile_macro::lang_profile;
use crate::profiles::Language;

lang_profile! {
    /// The Rust language profile (zero-sized).
    pub struct RustProfile {
        info: INFO,
        language: Language::Rust,
        extensions: &["rs"],
        grammar: tree_sitter_rust::LANGUAGE,
        walk: traversal::walk,
        root_for: roots::root_for,
        module_name: roots::module_name,
        is_package_index: roots::is_package_index,
        containing_package: resolve::containing_package,
        // `crate::`/`self::`/`super::`/package-name paths resolve on disk.
        resolve_target: |import, _pkg, file, root| resolve::resolve_target(import, file, root),
        // `use crate::noze::smells` — the last segment may be a submodule.
        submodule_candidate: |target, symbol| Some(format!("{target}/{symbol}")),
        decorators: false,
        classify_decorator: deadcode::classify,
        is_conventionally_private: deadcode::is_conventionally_private,
        is_entry_file_stem: deadcode::is_entry_file_stem,
        dead_code_defaults: deadcode::defaults,
        entry_modules: deadcode::entry_modules,
        expensive_loop_methods: performance::EXPENSIVE_LOOP_METHODS,
        external_get_receivers: performance::EXTERNAL_GET_RECEIVERS,
        // An edge into the importer's own subtree (`use self::builder::build`,
        // façade re-exports) is containment, like `mod builder;` itself.
        is_containment: |importer: &str, target: &str| {
            target
                .strip_prefix(importer)
                .is_some_and(|rest| rest.starts_with('/'))
        },
    }
}
