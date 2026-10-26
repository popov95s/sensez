use crate::report::SmellKind;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

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
