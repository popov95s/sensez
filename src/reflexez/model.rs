use serde::Serialize;
use std::path::PathBuf;

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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanReason {
    ChangedTest,
    DirectDependency,
    TransitiveDependency,
    DynamicImport,
    FullRequested,
    SafetyFallback,
}

#[derive(Clone, Debug, Serialize)]
pub struct Selection {
    pub file: PathBuf,
    pub runner: RunnerKind,
    pub reason: PlanReason,
    pub distance: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct RunnerPlan {
    pub kind: RunnerKind,
    pub root: PathBuf,
    pub program: PathBuf,
    pub prefix_args: Vec<String>,
    pub tests: Vec<PathBuf>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ImpactPlan {
    pub repository: PathBuf,
    pub changed_files: Vec<PathBuf>,
    pub discovered_tests: usize,
    pub selected: Vec<Selection>,
    pub runners: Vec<RunnerPlan>,
    pub full_suite: bool,
    pub fallback_reasons: Vec<String>,
    pub unresolved_dynamic_imports: usize,
    pub selection_ms: u128,
}
