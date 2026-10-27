//! Evidence-based confidence for a dead-code candidate.

use super::demand::Inbound;
use crate::report::Confidence;

/// What the finding is about beyond its importers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Reached {
    FromRoot,
    Orphan,
    /// Only test sources reach the module.
    TestsOnly,
}

/// Why a candidate is reported below High beyond its importers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DeadCodeReason {
    TestOnly,
    UnreachableModule,
}

impl DeadCodeReason {
    fn as_str(self) -> &'static str {
        match self {
            DeadCodeReason::TestOnly => "test_only",
            DeadCodeReason::UnreachableModule => "unreachable_module",
        }
    }
}

/// Facts that weaken "nothing uses this".
pub(super) struct Evidence<'a> {
    pub inbound: &'a Inbound,
    pub reached: Reached,
    pub is_default_export: bool,
    pub test_only: bool,
}

/// A candidate's confidence and the strongest reason behind it.
pub(super) struct Grade {
    pub confidence: Confidence,
    pub reason: Option<DeadCodeReason>,
}

impl Grade {
    /// The report's `reason` text; empty when there is none.
    pub(super) fn reason_text(&self) -> String {
        self.reason.map_or("", DeadCodeReason::as_str).to_string()
    }
}

/// High unless some evidence suggests hidden use.
pub(super) fn grade(evidence: &Evidence) -> Grade {
    if evidence.reached == Reached::Orphan {
        return medium(Some(DeadCodeReason::UnreachableModule));
    }
    if evidence.test_only || evidence.reached == Reached::TestsOnly {
        return medium(Some(DeadCodeReason::TestOnly));
    }
    let inbound = evidence.inbound;
    let weakened =
        evidence.is_default_export || inbound.opaque || inbound.type_star || inbound.from_config;
    if weakened {
        medium(None)
    } else {
        Grade {
            confidence: Confidence::High,
            reason: None,
        }
    }
}

fn medium(reason: Option<DeadCodeReason>) -> Grade {
    Grade {
        confidence: Confidence::Medium,
        reason,
    }
}
