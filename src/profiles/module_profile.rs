use super::import_resolver::ImportResolver;
use std::path::{Path, PathBuf};

pub trait ModuleProfile: Send + Sync {
    /// Workspace/package root containing `file` (module names are relative to it).
    fn root_for(&self, file: &Path) -> PathBuf;
    /// Logical module name for `file` under `root`.
    fn module_name(&self, file: &Path, root: &Path) -> String;
    /// True when `file` is the package's `__init__`/`index` entry.
    fn is_package_index(&self, file: &Path) -> bool;
    /// The containing package of `module_name` (`a.b.c` -> `a.b`; index files stay one level deeper).
    fn containing_package(&self, module_name: &str, is_index: bool) -> String;
    /// Fresh resolver state for one graph build.
    fn import_resolver(&self) -> Box<dyn ImportResolver>;
    /// Profile-specific, workspace-relative name for a colliding module identity.
    fn disambiguated_module_name(&self, file: &Path, workspace_root: &Path, base: &str) -> String {
        let _ = (file, workspace_root);
        base.to_string()
    }
    /// Path key matched against [`ImportResolver::target_path_keys`]; `None` for dotted namespaces.
    fn path_key(&self, _file: &Path) -> Option<PathBuf> {
        None
    }
    /// Workspace above `common` whose layout disambiguates colliding module names.
    fn workspace_root(&self, _common: &Path) -> Option<PathBuf> {
        None
    }
    fn submodule_candidate(&self, target: &str, symbol: &str) -> Option<String>;
    fn is_containment(&self, _importer: &str, _target: &str) -> bool {
        false
    }
}
