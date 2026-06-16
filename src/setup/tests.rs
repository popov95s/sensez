//! `sense init` artifact tests (non-interactive paths only).

use super::*;
use std::fs;

fn temp_root() -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().canonicalize().unwrap();
    (tmp, root)
}

#[test]
fn flag_driven_init_writes_all_artifacts() {
    let (_tmp, root) = temp_root();
    run(InitOptions {
        path: Some(root.clone()),
        agent: Some("claude-code".into()),
        gate: true,
        no_metrics: true,
        yes: true,
    })
    .unwrap();

    let toml = fs::read_to_string(root.join("sensez.toml")).unwrap();
    assert!(toml.contains("[self_improvement]"));
    assert!(toml.contains("enabled = false"), "--no-metrics honored");

    let mcp: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root.join(".mcp.json")).unwrap()).unwrap();
    assert_eq!(mcp["mcpServers"]["sense"]["args"][0], "mcp");
    assert_eq!(mcp["mcpServers"]["sense"]["args"][1], "serve");

    let settings: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root.join(".claude/settings.json")).unwrap())
            .unwrap();
    let stop = settings["hooks"]["Stop"].to_string();
    assert!(
        stop.contains("mcp_tool"),
        "gate runs via the warm MCP server"
    );
    assert!(stop.contains("stop_hook_active"), "soft-block escape wired");

    assert!(fs::read_to_string(root.join(".gitignore"))
        .unwrap()
        .contains(".sensez/"));

    // Idempotent: a second run must not duplicate the hook entry.
    run(InitOptions {
        path: Some(root.clone()),
        agent: Some("claude-code".into()),
        gate: true,
        no_metrics: false,
        yes: true,
    })
    .unwrap();
    let settings: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root.join(".claude/settings.json")).unwrap())
            .unwrap();
    assert_eq!(settings["hooks"]["Stop"].as_array().unwrap().len(), 1);
}

#[test]
fn pyproject_mode_appends_tool_sensez() {
    let (_tmp, root) = temp_root();
    fs::write(root.join("pyproject.toml"), "[project]\nname = \"x\"\n").unwrap();
    let msg = artifacts::write_config(&root, true, true).unwrap();
    assert!(msg.contains("pyproject"));

    let text = fs::read_to_string(root.join("pyproject.toml")).unwrap();
    assert!(text.contains("[tool.sensez.self_improvement]"));
    assert!(text.contains("[tool.sensez.duplication]"));
    // Round-trip: the config layer must actually parse what init wrote.
    let cfg = crate::config::model::Config::load(&root).unwrap();
    assert!(cfg.self_improvement.enabled);
}

#[test]
fn no_tty_no_flags_refuses_instead_of_hanging() {
    let (_tmp, root) = temp_root();
    let result = run(InitOptions {
        path: Some(root.clone()),
        agent: None,
        gate: false,
        no_metrics: false,
        yes: false,
    });
    // Test runners have no TTY, so this must bail with guidance.
    assert!(result.is_err());
}
