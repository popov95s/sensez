//! Command string an MCP config stores for this sensez install.

use super::agents::McpLaunchDir;
use super::scope::InstallScope;
use std::path::{Path, PathBuf};

pub(super) const PROJECT_SHIM: &str = if cfg!(windows) {
    "node_modules/.bin/sensez.cmd"
} else {
    "node_modules/.bin/sensez"
};
const PATH_NAMES: &[&str] = if cfg!(windows) {
    &["sensez.exe", "sensez.cmd"]
} else {
    &["sensez"]
};
const NPM_WRAPPER: &str = "sensez/bin/sensez.js";

/// How an MCP config launches sensez.
#[derive(Debug, PartialEq, Eq)]
pub enum McpCommand {
    ProjectShim,
    OnPath,
    Absolute(PathBuf),
}

impl McpCommand {
    /// The command string stored in the MCP config.
    pub fn as_config_value(&self) -> String {
        match self {
            McpCommand::ProjectShim => PROJECT_SHIM.to_string(),
            McpCommand::OnPath => "sensez".to_string(),
            McpCommand::Absolute(exe) => exe.display().to_string(),
        }
    }
}

/// Project shim, then bare `sensez` on PATH, then the absolute executable path.
pub fn mcp_command(
    root: &Path,
    scope: InstallScope,
    launch_dir: McpLaunchDir,
    exe: &Path,
) -> McpCommand {
    let in_project_modules = exe.starts_with(root.join("node_modules"));
    if uses_project_shim(root, scope, launch_dir, in_project_modules) {
        return McpCommand::ProjectShim;
    }
    if !in_project_modules && sensez_on_path().is_some_and(|found| launches(&found, exe)) {
        return McpCommand::OnPath;
    }
    McpCommand::Absolute(exe.to_path_buf())
}

fn uses_project_shim(
    root: &Path,
    scope: InstallScope,
    launch_dir: McpLaunchDir,
    in_project_modules: bool,
) -> bool {
    scope == InstallScope::Project
        && launch_dir == McpLaunchDir::WorkspaceRoot
        && in_project_modules
        && root.join(PROJECT_SHIM).exists()
}

/// Whether running `on_path` ends up executing `exe`, directly or via the npm wrapper.
pub(super) fn launches(on_path: &Path, exe: &Path) -> bool {
    same_file(on_path, exe) || is_npm_wrapper_for(on_path, exe) || is_npm_cmd_shim_for(on_path, exe)
}

/// npm on Windows puts `sensez.cmd` next to the `node_modules` holding the wrapper.
fn is_npm_cmd_shim_for(on_path: &Path, exe: &Path) -> bool {
    on_path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("cmd"))
        && is_npm_wrapper_for(
            &on_path.with_file_name("node_modules").join(NPM_WRAPPER),
            exe,
        )
}

fn is_npm_wrapper_for(on_path: &Path, exe: &Path) -> bool {
    exe.ancestors()
        .filter(|dir| dir.file_name().is_some_and(|name| name == "node_modules"))
        .any(|node_modules| same_file(on_path, &node_modules.join(NPM_WRAPPER)))
}

fn sensez_on_path() -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .flat_map(|dir| PATH_NAMES.iter().map(move |name| dir.join(name)))
        .find(|candidate| candidate.is_file())
}

fn same_file(a: &Path, b: &Path) -> bool {
    matches!((a.canonicalize(), b.canonicalize()), (Ok(a), Ok(b)) if a == b)
}
