//! Records how the MCP server is used and how often its findings lead to real fixes.

mod aging;
mod events;
pub(crate) mod file_lock;
mod fingerprint;
mod flush;
mod gate_memory;
mod hooks;
mod hub;
mod json_file;
mod ranking;
mod recapture;
mod recapture_state;
mod recording;
mod report;
mod staleness;
mod store;
#[cfg(test)]
mod store_tests;
mod suppression;
mod triage;
mod usage;
mod verdict;

pub use events::Origin;
pub use flush::flush;
pub use gate_memory::{finding_identities, retain_unseen_gate_findings};
pub(crate) use hooks::observe_source_state;
pub use hooks::recapture;
pub use ranking::{rank_by_precision, regressions};
#[cfg(feature = "eyez")]
pub use recording::record_search;
pub use recording::{record_gate_block, record_scan, triage_finding};

pub use suppression::apply_suppressions;
pub use usage::usage_report;
pub use verdict::{TriageVerdict, VERDICTS};
