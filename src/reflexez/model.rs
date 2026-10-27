use serde::Serialize;
use std::path::PathBuf;

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

impl PlanReason {
    pub fn label(self) -> &'static str {
        match self {
            Self::ChangedTest => "changed_test",
            Self::DirectDependency => "direct_dependency",
            Self::TransitiveDependency => "transitive_dependency",
            Self::DynamicImport => "dynamic_import",
            Self::FullRequested => "full_requested",
            Self::SafetyFallback => "safety_fallback",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunMode {
    Narrowed,
    Full,
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
    pub mode: RunMode,
    pub root: PathBuf,
    pub program: PathBuf,
    pub prefix_args: Vec<String>,
    pub tests: Vec<PathBuf>,
}

pub use crate::profiles::runner_config::{ConfigWarning, RunnerKind};

#[derive(Clone, Debug, Serialize)]
pub struct ImpactPlan {
    pub repository: PathBuf,
    pub changed_files: Vec<PathBuf>,
    pub discovered_tests: usize,
    pub selected: Vec<Selection>,
    pub runners: Vec<RunnerPlan>,
    pub full_suite: bool,
    pub fallback_reasons: Vec<String>,
    pub config_warnings: Vec<ConfigWarning>,
    pub settings_warnings: Vec<String>,
    pub unresolved_dynamic_imports: usize,
    pub opaque_import_files: Vec<PathBuf>,
    pub unmapped_changes: Vec<PathBuf>,
    pub selection_ms: u128,
}
