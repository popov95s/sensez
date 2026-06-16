//! Records how the MCP server is used and how often its findings lead to real
//! fixes. Everything stays on disk under each repo's `.sensez/local-metrics/`.

mod events;
mod flush;
mod hub;
mod ranking;
mod recapture;
mod recording;
mod report;
mod resolve;
mod store;
mod suppression;
mod triage;
mod usage;

pub use events::Origin;
pub use flush::flush;
pub use ranking::{rank_by_precision, regressions};
#[cfg(feature = "eyez")]
pub use recording::record_search;
pub use recording::{record_gate_block, record_scan, triage_finding, BaselineUpdate};
pub use suppression::apply_suppressions;
pub use usage::usage_report;

pub fn recapture() {
    recapture::run();
}
