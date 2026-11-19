//! Language profile traits and the shared detectors built on them.

pub(crate) mod branch_path;
pub(crate) mod comment_class;
pub(crate) mod comments;
pub(crate) mod conditionals;
mod decorator_class;
pub mod dependencies;
#[cfg(any(feature = "lang-javascript", feature = "lang-python"))]
pub(crate) mod echo_assertion;
#[cfg(any(feature = "lang-javascript", feature = "lang-python"))]
pub(crate) mod fallback_guard;
pub(crate) mod for_loop;
pub(crate) mod guard_clause;
pub(crate) mod guard_fingerprint;
#[cfg(any(feature = "lang-javascript", feature = "lang-python"))]
pub(crate) mod handler_end;
pub(crate) mod lazy_init;
pub(crate) mod lexeme;
pub(crate) mod lineage;
#[cfg(any(feature = "lang-javascript", feature = "lang-python"))]
pub(crate) mod magic_number;
pub(crate) mod node_names;
pub(crate) mod node_query;
#[cfg(any(feature = "lang-javascript", feature = "lang-rust"))]
pub(crate) mod pathroot;
pub mod registry;
pub(crate) mod runner_config;
#[cfg(test)]
pub(crate) mod test_fixtures;
pub(crate) mod test_oracle;
#[cfg(any(feature = "lang-javascript", feature = "lang-python"))]
pub(crate) mod todo_comment;
#[cfg(any(feature = "lang-javascript", feature = "lang-python"))]
pub(crate) mod try_handling;
pub mod typevocab;
pub mod utility_lookalikes;
pub(crate) mod walk;
pub mod wording;

#[cfg(feature = "lang-javascript")]
pub mod javascript;
#[cfg(feature = "lang-python")]
pub mod python;
#[cfg(feature = "lang-rust")]
pub mod rust;
#[cfg(feature = "lang-typescript")]
pub mod typescript;

mod compile_globs;
mod dead_code_defaults;
mod dead_code_profile;
mod entry_candidate;
mod external_bases;
mod import_resolver;
mod language_info;
mod language_profile;
mod module_profile;
mod parse_profile;
mod performance_profile;
mod type_vocabulary_profile;

pub(crate) use compile_globs::compile_profile_globs;
pub use dead_code_defaults::DeadCodeDefaults;
pub use dead_code_profile::DeadCodeProfile;
pub use decorator_class::DecoratorClass;
pub use dependencies::DependencyProfile;
pub use entry_candidate::EntryCandidate;
pub use external_bases::ExternalBases;
pub use import_resolver::ImportResolver;
pub use language_info::LanguageInfo;
pub use language_profile::LanguageProfile;
pub use module_profile::ModuleProfile;
pub use parse_profile::ParseProfile;
pub use performance_profile::PerformanceProfile;
pub use type_vocabulary_profile::TypeVocabularyProfile;

use crate::spine::ir::Language;
