use serde::Serialize;
use std::path::PathBuf;

/// A runner-config key that could not be read statically.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ConfigWarning {
    pub file: PathBuf,
    pub key: String,
    pub reason: String,
    pub fallback: String,
}

/// A runner-configured setup module; changes to it or its imports affect every test.
#[derive(Clone, Debug)]
pub struct SetupFile {
    pub path: PathBuf,
    pub config: PathBuf,
    pub key: String,
}

/// A test runner reflexez can plan for.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunnerKind {
    Pytest,
    Vitest,
    Jest,
}

impl RunnerKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Pytest => "pytest",
            Self::Vitest => "vitest",
            Self::Jest => "jest",
        }
    }
}
