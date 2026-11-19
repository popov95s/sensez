//! Code-health analysis: cycles, dead code, duplication, and smells.

mod analyze;
pub mod cycles;
pub mod dead_code;
pub mod duplication;
pub mod glossary;
mod limit;
mod parallel;
pub mod smells;
mod source_scope;

#[cfg(test)]
pub use analyze::run;
pub use analyze::run_with_root;
pub use limit::limit;
