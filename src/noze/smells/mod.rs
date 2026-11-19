//! Pillar 5: design-smell detection.

mod clumps;
mod cohesion;
mod complexity;
mod context;
mod coupling;
mod dependency_reimplementation;
mod detect;
mod exclusive_paths;
mod external_vocab;
mod finalize;
mod finding;
mod fixture_echo;
mod graphy;
mod implicit_schema;
mod inherit;
mod lines;
mod loose_fields;
mod magic_string;
mod measure;
mod mock_modules;
mod mocked_subject;
mod mutation;
mod name_promise;
mod name_promise_writes;
mod performance;
mod review_risks;
mod sibling_helpers;
mod size;
mod structural;
mod stub_shipped;
mod swallowed_error;
mod type_escape;
mod typing;
mod union_find;

#[cfg(test)]
mod cohesion_tests;
#[cfg(test)]
mod coupling_tests;
#[cfg(test)]
mod defensive_code_tests;
#[cfg(test)]
mod dependency_reimplementation_tests;
#[cfg(test)]
mod fixture_echo_tests;
#[cfg(test)]
mod god_module_tests;
#[cfg(test)]
mod graph_tests;
#[cfg(test)]
mod mocked_subject_tests;
#[cfg(test)]
mod review_risks_tests;
#[cfg(test)]
mod schema_tests;
#[cfg(test)]
mod sibling_helpers_tests;
#[cfg(test)]
mod structural_tests;
#[cfg(test)]
mod stub_shipped_tests;
#[cfg(test)]
mod test_oracle_tests;
#[cfg(test)]
pub(crate) mod test_support;
#[cfg(test)]
mod type_escape_tests;

pub(super) use context::SmellContext;
pub use detect::detect;
#[cfg(test)]
pub use detect::detect_local;
pub(super) use finding::make;
use measure::Measure;

#[cfg(test)]
mod comments_tests;
#[cfg(test)]
mod determinism_tests;
#[cfg(test)]
mod discipline_tests;
#[cfg(test)]
mod line_anchor_tests;
#[cfg(test)]
mod literal_membership_tests;
#[cfg(test)]
mod loop_frame_tests;
#[cfg(test)]
mod loose_typing_tests;
#[cfg(test)]
mod magic_string_sink_tests;
#[cfg(test)]
mod magic_string_tests;
#[cfg(test)]
mod message_chain_tests;
#[cfg(test)]
mod mutated_parameter_tests;
#[cfg(test)]
mod name_promise_tests;
#[cfg(test)]
mod nested_if_tests;
#[cfg(test)]
mod nested_ternary_tests;
#[cfg(test)]
mod performance_tests;
#[cfg(test)]
mod repeated_iteration_tests;
#[cfg(test)]
mod swallowed_error_tests;
#[cfg(test)]
mod tests;
