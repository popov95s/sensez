//! MCP launch command tests.

use super::tests::temp_root;
use super::*;
use std::fs;
use std::path::{Path, PathBuf};

fn npm_install(root: &Path) -> PathBuf {
    let exe = root.join("node_modules/sensez-x/bin/sensez");
    fs::create_dir_all(exe.parent().unwrap()).unwrap();
    fs::create_dir_all(root.join("node_modules/.bin")).unwrap();
    fs::write(&exe, "").unwrap();
    fs::write(root.join(command::PROJECT_SHIM), "").unwrap();
    exe
}

#[test]
fn mcp_command_prefers_project_shim() {
    let (_tmp, root) = temp_root();
    let exe = npm_install(&root);
    let command = command::mcp_command(
        &root,
        InstallScope::Project,
        agents::McpLaunchDir::WorkspaceRoot,
        &exe,
    );
    assert_eq!(command, command::McpCommand::ProjectShim);
}

#[test]
fn mcp_command_skips_shim_when_client_launch_dir_is_unknown() {
    let (_tmp, root) = temp_root();
    let exe = npm_install(&root);
    let command = command::mcp_command(
        &root,
        InstallScope::Project,
        agents::McpLaunchDir::Unknown,
        &exe,
    );
    assert_eq!(command, command::McpCommand::Absolute(exe));
}

#[test]
fn mcp_command_uses_absolute_for_global_scope() {
    let (_tmp, root) = temp_root();
    let exe = npm_install(&root);
    let command = command::mcp_command(
        &root,
        InstallScope::Global,
        agents::McpLaunchDir::WorkspaceRoot,
        &exe,
    );
    assert_eq!(command, command::McpCommand::Absolute(exe));
}

#[cfg(unix)]
#[test]
fn global_npm_wrapper_on_path_launches_platform_binary() {
    let tmp = tempfile::tempdir().unwrap();
    let prefix = tmp.path().canonicalize().unwrap();
    let modules = prefix.join("lib/node_modules");
    let wrapper = modules.join("sensez/bin/sensez.js");
    let exe = modules.join("sensez-x/bin/sensez");
    for file in [&wrapper, &exe] {
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, "").unwrap();
    }
    fs::create_dir_all(prefix.join("bin")).unwrap();
    let on_path = prefix.join("bin/sensez");
    std::os::unix::fs::symlink("../lib/node_modules/sensez/bin/sensez.js", &on_path).unwrap();
    let other = tempfile::tempdir().unwrap();
    let foreign = npm_install(other.path());

    assert!(command::launches(&on_path, &exe));
    assert!(!command::launches(&on_path, &foreign));
}

#[test]
fn global_npm_cmd_shim_launches_platform_binary() {
    let tmp = tempfile::tempdir().unwrap();
    let prefix = tmp.path().canonicalize().unwrap();
    let modules = prefix.join("node_modules");
    let exe = modules.join("sensez-x/bin/sensez.exe");
    for file in [
        &modules.join("sensez/bin/sensez.js"),
        &exe,
        &prefix.join("sensez.cmd"),
    ] {
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, "").unwrap();
    }

    assert!(command::launches(&prefix.join("sensez.cmd"), &exe));
}

#[test]
fn init_writes_no_absolute_path_under_project_node_modules() {
    let (_tmp, root) = temp_root();
    npm_install(&root);
    run(InitOptions {
        path: Some(root.clone()),
        agent: Some("claude-code".into()),
        gate: false,
        no_metrics: true,
        yes: true,
    })
    .unwrap();

    let mcp = fs::read_to_string(root.join(".mcp.json")).unwrap();
    let node_modules = root.join("node_modules").display().to_string();
    assert!(!mcp.contains(&node_modules), "{mcp}");
}
