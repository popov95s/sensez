//! Per-language design-smell configuration.

mod compiled;
mod defaults;
mod knob_table;
mod knobs;
mod raw;
mod resolve;
mod rule_lists;
mod rules;
mod smell_config;
mod strictness;
mod toggles;

pub use knobs::{Smells, Strictness};
pub(crate) use rules::rule_keys;
pub use smell_config::SmellConfig;

#[cfg(test)]
mod flat_knob_tests;
#[cfg(test)]
mod knob_type_tests;
#[cfg(test)]
mod noise_rule_tests;
#[cfg(test)]
mod serde_tests;
#[cfg(test)]
mod tests;
