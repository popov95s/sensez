use super::model::RunnerKind;
use std::path::Path;

pub use crate::profiles::runner_config::SetupFile;

#[cfg(feature = "lang-javascript")]
pub use crate::profiles::javascript::runner_config::RunnerConfig;

#[cfg(not(feature = "lang-javascript"))]
#[derive(Default)]
pub struct RunnerConfig {
    pub setup_files: Vec<SetupFile>,
    pub warnings: Vec<crate::profiles::runner_config::ConfigWarning>,
}

#[cfg(not(feature = "lang-javascript"))]
impl RunnerConfig {
    pub fn is_test(&self, _file: &Path) -> bool {
        false
    }

    pub fn project_dirs(&self) -> impl Iterator<Item = &Path> {
        std::iter::empty()
    }
}

#[cfg(feature = "lang-javascript")]
pub fn load(root: &Path, runner: RunnerKind) -> RunnerConfig {
    use crate::profiles::javascript::runner_config::{jest, vitest};
    match runner {
        RunnerKind::Vitest => vitest::read(root),
        RunnerKind::Jest => jest::read(root),
        RunnerKind::Pytest => RunnerConfig::default(),
    }
}

#[cfg(not(feature = "lang-javascript"))]
pub fn load(_root: &Path, _runner: RunnerKind) -> RunnerConfig {
    RunnerConfig::default()
}

#[cfg(feature = "lang-javascript")]
pub use crate::profiles::javascript::runner_config::detect::{config_runner, manifest_runners};

#[cfg(not(feature = "lang-javascript"))]
pub fn manifest_runners(_manifest: &serde_json::Value) -> Vec<RunnerKind> {
    Vec::new()
}

#[cfg(not(feature = "lang-javascript"))]
pub fn config_runner(_root: &Path) -> Option<RunnerKind> {
    None
}
