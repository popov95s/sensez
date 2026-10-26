use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// A human verdict on a finding; `Clear` removes an earlier one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriageVerdict {
    Debt,
    FalsePositive,
    Clear,
}

impl TriageVerdict {
    pub const ALL: [TriageVerdict; 3] = [
        TriageVerdict::Debt,
        TriageVerdict::FalsePositive,
        TriageVerdict::Clear,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            TriageVerdict::Debt => "debt",
            TriageVerdict::FalsePositive => "false_positive",
            TriageVerdict::Clear => "clear",
        }
    }

    pub fn from_name(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|verdict| verdict.as_str() == value)
    }
}

impl Display for TriageVerdict {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Accepted `verdict` names, in [`TriageVerdict::ALL`] order.
pub const VERDICTS: [&str; 3] = [
    TriageVerdict::Debt.as_str(),
    TriageVerdict::FalsePositive.as_str(),
    TriageVerdict::Clear.as_str(),
];
