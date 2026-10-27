//! Pytest collection rules, read by the Python profile.

#[cfg(feature = "lang-python")]
pub use crate::profiles::python::pytest_config::PytestConfigs;

#[cfg(not(feature = "lang-python"))]
pub use without_python::PytestConfigs;

#[cfg(not(feature = "lang-python"))]
mod without_python {
    use std::path::{Path, PathBuf};

    /// A rootdir that collects nothing: Python files are not sources without the profile.
    pub struct PytestConfig {
        pub dir: PathBuf,
    }

    impl PytestConfig {
        pub fn collects(&self, _path: &Path) -> bool {
            false
        }
    }

    pub struct PytestConfigs(PytestConfig);

    impl PytestConfigs {
        pub fn new(repository: &Path) -> Self {
            Self(PytestConfig {
                dir: repository.to_path_buf(),
            })
        }

        pub fn for_file(&mut self, _file: &Path) -> &PytestConfig {
            &self.0
        }
    }
}
