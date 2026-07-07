use crate::report::SmellKind;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Namespace {
    Cycles,
    DeadCode,
    Boundaries,
    Duplication,
    Smells,
}

impl Namespace {
    pub fn as_str(self) -> &'static str {
        match self {
            Namespace::Cycles => "cycles",
            Namespace::DeadCode => "dead_code",
            Namespace::Boundaries => "boundaries",
            Namespace::Duplication => "duplication",
            Namespace::Smells => "smells",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "cycles" => Some(Namespace::Cycles),
            "dead_code" => Some(Namespace::DeadCode),
            "boundaries" => Some(Namespace::Boundaries),
            "duplication" => Some(Namespace::Duplication),
            "smells" => Some(Namespace::Smells),
            _ => None,
        }
    }
}

impl Display for Namespace {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Detector {
    Cycles,
    DeadCode { symbol_kind: String },
    Boundaries,
    Duplication,
    Smell { smell: SmellKind },
}

impl Display for Detector {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Detector::Cycles => f.write_str("cycles"),
            Detector::DeadCode { symbol_kind } => write!(f, "dead_code/{symbol_kind}"),
            Detector::Boundaries => f.write_str("boundaries"),
            Detector::Duplication => f.write_str("duplication"),
            Detector::Smell { smell } => write!(f, "smells/{smell}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Label {
    Cycle {
        modules: String,
    },
    DeadCode {
        module: String,
        symbol: String,
        symbol_kind: String,
    },
    Boundary {
        from: String,
        to: String,
        rule: String,
    },
    Clone {
        arity: usize,
        files: String,
    },
    Smell {
        smell: SmellKind,
        file: String,
        symbol: String,
    },
}

impl Display for Label {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Label::Cycle { modules } => f.write_str(modules),
            Label::DeadCode {
                module,
                symbol,
                symbol_kind,
            } => write!(f, "{module}::{symbol} ({symbol_kind})"),
            Label::Boundary { from, to, rule } => write!(f, "{from} -> {to} ({rule})"),
            Label::Clone { arity, files } => write!(f, "clone x{arity}: {files}"),
            Label::Smell {
                smell,
                file,
                symbol,
            } => write!(f, "{smell} @ {file}::{symbol}"),
        }
    }
}
