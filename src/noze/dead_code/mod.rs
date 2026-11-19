//! Pillar 2: unreferenced-symbol detection.

mod class_entrypoints;
mod confidence;
mod demand;
mod extra;
mod findings;
mod loaded_dirs;
mod manifest_callees;
mod modules;
mod reach;
mod reachability;
mod roots;
mod rule_sets;

pub use findings::detect;
use rule_sets::{RuleSet, RuleSets};

#[cfg(test)]
mod member_tests;
#[cfg(test)]
mod test_sources;
#[cfg(test)]
mod tests;
