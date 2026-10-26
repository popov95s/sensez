use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// One of the five finding families a report is split into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pillar {
    Cycles,
    DeadCode,
    Boundaries,
    Duplication,
    Smells,
}

impl Pillar {
    pub const ALL: [Pillar; 5] = [
        Pillar::Cycles,
        Pillar::DeadCode,
        Pillar::Boundaries,
        Pillar::Duplication,
        Pillar::Smells,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Pillar::Cycles => "cycles",
            Pillar::DeadCode => "dead_code",
            Pillar::Boundaries => "boundaries",
            Pillar::Duplication => "duplication",
            Pillar::Smells => "smells",
        }
    }

    pub fn from_name(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|pillar| pillar.as_str() == value)
    }
}

impl Display for Pillar {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
