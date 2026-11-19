//! Pillar 1: copy-paste duplication via a generalized suffix array.

mod advice;
mod class_shapes;
mod clones;
mod consolidate;
mod detect;
mod file_units;
mod flatten;
#[cfg(test)]
mod gap_tests;
mod gapped;
mod lexeme_similarity;
mod near_miss;
mod occurrences;
mod open_tail;
#[cfg(test)]
mod range_tests;
#[cfg(feature = "eyez")]
mod semantic;
#[cfg(test)]
pub(crate) mod test_support;

#[cfg(test)]
pub use detect::detect;
pub use detect::detect_with_root;
pub(super) use file_units::{span_index_range, top_level_functions};
pub(crate) use lexeme_similarity::{function_lexemes, length_ceiling_percent, similarity_percent};

#[cfg(test)]
mod tests;
