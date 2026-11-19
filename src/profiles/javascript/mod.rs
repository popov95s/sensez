//! JavaScript language profile.

mod chains;
pub(crate) mod classunit;
mod comment_runs;
pub(crate) mod comment_syntax;
mod compared_defaults;
pub(crate) mod conditionals;
pub(crate) mod const_eval;
mod control_shapes;
pub(crate) mod deadcode;
pub(crate) mod declaration;
pub(crate) mod echo_facts;
pub(crate) mod fallback_sink;
pub(crate) mod generated;
pub(crate) mod grammar;
pub(crate) mod imports;
mod iteration;
pub(crate) mod lexeme;
mod local_functions;
pub(crate) mod loops;
mod magic_numbers;
pub(crate) mod manifest;
mod member_visibility;
mod membership;
mod mock_targets;
mod namespaces;
mod nesting;
pub(crate) mod obsession;
mod params;
pub(crate) mod performance;
pub(crate) mod programs;
pub(crate) mod resolve;
mod returned_names;
pub(crate) mod risk_facts;
pub(crate) mod roots;
pub(crate) mod runner_config;
mod runtime_guard;
pub(crate) mod scope;
mod state_writes;
mod stub_body;
pub(crate) mod symbols;
pub(crate) mod test_support;
pub(crate) mod tokens;
pub(crate) mod traversal;
mod type_escapes;
pub(crate) mod typehints;
pub(crate) mod typespace;
pub(crate) mod typevocab;
pub(crate) mod units;
pub(crate) mod utilities;
pub(crate) mod wording;
mod workflows;

#[cfg(test)]
mod fused_collector_tests;
#[cfg(test)]
mod tests;

mod profile;

pub use profile::JsProfile;
