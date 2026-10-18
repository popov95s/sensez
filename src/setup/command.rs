//! Command string an MCP config stores for this sensez install.

use super::agents::McpLaunchDir;
use super::scope::InstallScope;
use std::path::{Path, PathBuf};

const PROJECT_SHIM: &str = "node_modules/.bin/sensez";
const NPM_WRAPPER: &str = "sensez/bin/sensez.js";

/// Project shim, then bare `sensez` on PATH, then the absolute executable path.
pub fn mcp_command(
    root: &Path,
    scope: InstallScope,
    launch_dir: McpLaunchDir,
    exe: &Path,
) -> String {
    let in_project_modules = exe.starts_with(root.join("node_modules"));
    if scope == InstallScope::Project
        && launch_dir == McpLaunchDir::WorkspaceRoot
        && in_project_modules
        && root.join(PROJECT_SHIM).exists()
    {
        return PROJECT_SHIM.to_string();
    }
    if !in_project_modules && sensez_on_path().is_some_and(|found| launches(&found, exe)) {
        return "sensez".to_string();
    }
    exe.display().to_string()
}

/// Whether running `on_path` ends up executing `exe`, directly or via the npm wrapper.
pub(super) fn launches(on_path: &Path, exe: &Path) -> bool {
    same_file(on_path, exe) || is_npm_wrapper_for(on_path, exe)
}

fn is_npm_wrapper_for(on_path: &Path, exe: &Path) -> bool {
    exe.ancestors()
        .filter(|dir| dir.file_name().is_some_and(|name| name == "node_modules"))
        .any(|node_modules| same_file(on_path, &node_modules.join(NPM_WRAPPER)))
}

fn sensez_on_path() -> Option<PathBuf> {
    let name = format!("sensez{}", std::env::consts::EXE_SUFFIX);
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|dir| dir.join(&name))
        .find(|candidate| candidate.is_file())
}

fn same_file(a: &Path, b: &Path) -> bool {
    matches!((a.canonicalize(), b.canonicalize()), (Ok(a), Ok(b)) if a == b)
}
