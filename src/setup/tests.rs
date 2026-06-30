//! `sensez init` artifact tests (non-interactive paths only).

use super::*;
use std::fs;
use std::path::PathBuf;

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
    assert_eq!(mcp["mcpServers"]["sensez"]["args"][0], "mcp");
    assert_eq!(mcp["mcpServers"]["sensez"]["args"][1], "serve");

    let settings: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root.join(".claude/settings.json")).unwrap())
            .unwrap();
    let stop = settings["hooks"]["Stop"].to_string();
    assert!(
        stop.contains("mcp_tool"),
        "gate runs via the warm MCP server"
    );
    assert!(stop.contains("stop_hook_active"), "soft-block escape wired");
    assert!(
        stop.contains("experimental stop hook"),
        "hook setup should call out that it is experimental"
    );

    assert!(fs::read_to_string(root.join(".gitignore"))
        .unwrap()
        .contains(".sensez/"));
    assert!(root.join(".sensez").is_dir(), ".sensez dir created");
    assert!(root.join(".sensez/.gitignore").exists());

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
    let mcp: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root.join(".mcp.json")).unwrap()).unwrap();
    assert_eq!(mcp["mcpServers"].as_object().unwrap().len(), 1);
}

#[test]
fn interactive_agent_catalog_includes_more_modern_agents() {
    let ids: Vec<&str> = agents::AGENTS.iter().map(|a| a.id).collect();
    assert!(ids.contains(&"cline"));
    assert!(ids.contains(&"codex"));
    assert!(ids.contains(&"opencode"));
    assert!(ids.contains(&"pi"));
    assert_eq!(
        agents::find("codex").unwrap().mcp_relpath,
        Some(".codex/config.toml")
    );
    assert_eq!(
        agents::find("opencode").unwrap().mcp_relpath,
        Some("opencode.jsonc")
    );
}

#[test]
fn codex_has_project_config_path_and_can_fallback() {
    let spec = agents::find("codex").unwrap();
    assert_eq!(spec.mcp_relpath, Some(".codex/config.toml"));
}

#[test]
fn codex_mcp_config_uses_lowercase_table_name() {
    let (_tmp, root) = temp_root();
    artifacts::write_mcp_config(&root, "codex", "/bin/sensez").unwrap();

    let text = fs::read_to_string(root.join(".codex/config.toml")).unwrap();
    let config: toml::Value = toml::from_str(&text).unwrap();
    let sensez = &config["mcp_servers"]["sensez"];
    assert_eq!(sensez["command"].as_str(), Some("/bin/sensez"));
    let args: Vec<_> = sensez["args"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect();
    assert_eq!(args, vec!["mcp", "serve"]);
    assert!(!text.contains("[MCP_servers.legacy]"));
}

#[test]
fn codex_init_is_idempotent_and_preserves_existing_mcp_table() {
    let (_tmp, root) = temp_root();
    fs::create_dir_all(root.join(".codex")).unwrap();
    fs::write(
        root.join(".codex/config.toml"),
        "[MCP_servers.legacy]\ncommand = \"/old/legacy\"\nargs = [\"mcp\", \"serve\"]\n",
    )
    .unwrap();

    for _ in 0..2 {
        run(InitOptions {
            path: Some(root.clone()),
            agent: Some("codex".into()),
            gate: false,
            no_metrics: true,
            yes: true,
        })
        .unwrap();
    }

    let text = fs::read_to_string(root.join(".codex/config.toml")).unwrap();
    assert_eq!(text.matches("[mcp_servers.sensez]").count(), 1);
    assert!(text.contains("[MCP_servers.legacy]"));
}

#[test]
fn all_mcp_config_writers_are_idempotent() {
    for agent in agents::AGENTS
        .iter()
        .filter(|agent| agent.mcp_relpath.is_some())
    {
        let (_tmp, root) = temp_root();
        let path = root.join(agent.mcp_relpath.unwrap());
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("toml") => fs::write(
                &path,
                "[mcp_servers.other]\ncommand = \"/bin/other\"\nargs = []\n",
            )
            .unwrap(),
            Some("jsonc") => fs::write(
                &path,
                r#"{"$schema":"https://opencode.ai/config.json","mcp":{"other":{"type":"local","command":["/bin/other"],"enabled":true}}}"#,
            )
            .unwrap(),
            _ => fs::write(
                &path,
                r#"{"mcpServers":{"other":{"command":"/bin/other","args":[]}}}"#,
            )
            .unwrap(),
        }

        artifacts::write_mcp_config(&root, agent.id, "/bin/sensez").unwrap();
        artifacts::write_mcp_config(&root, agent.id, "/bin/sensez").unwrap();

        let text = fs::read_to_string(&path).unwrap();
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("toml") => {
                let config: toml::Value = toml::from_str(&text).unwrap();
                let servers = config["mcp_servers"].as_table().unwrap();
                assert!(
                    servers.contains_key("sensez"),
                    "missing sensez for {}",
                    agent.id
                );
                assert!(
                    servers.contains_key("other"),
                    "dropped other for {}",
                    agent.id
                );
                assert_eq!(text.matches("[mcp_servers.sensez]").count(), 1);
            }
            Some("jsonc") => {
                let config: serde_json::Value = serde_json::from_str(&text).unwrap();
                let servers = config["mcp"].as_object().unwrap();
                assert!(
                    servers.contains_key("sensez"),
                    "missing sensez for {}",
                    agent.id
                );
                assert!(
                    servers.contains_key("other"),
                    "dropped other for {}",
                    agent.id
                );
                assert_eq!(servers.len(), 2, "duplicate server for {}", agent.id);
            }
            _ => {
                let config: serde_json::Value = serde_json::from_str(&text).unwrap();
                let servers = config["mcpServers"].as_object().unwrap();
                assert!(
                    servers.contains_key("sensez"),
                    "missing sensez for {}",
                    agent.id
                );
                assert!(
                    servers.contains_key("other"),
                    "dropped other for {}",
                    agent.id
                );
                assert_eq!(servers.len(), 2, "duplicate server for {}", agent.id);
            }
        }
    }
}

#[test]
fn codex_init_preserves_other_existing_servers() {
    let (_tmp, root) = temp_root();
    fs::create_dir_all(root.join(".codex")).unwrap();
    fs::write(
        root.join(".codex/config.toml"),
        "[MCP_servers.legacy]\ncommand = \"/old/legacy\"\nargs = []\n\n\
         [MCP_servers.other]\ncommand = \"/bin/other\"\nargs = []\n",
    )
    .unwrap();

    artifacts::write_mcp_config(&root, "codex", "/bin/sensez").unwrap();

    let text = fs::read_to_string(root.join(".codex/config.toml")).unwrap();
    let config: toml::Value = toml::from_str(&text).unwrap();
    assert!(config["mcp_servers"]["sensez"].is_table());
    let legacy = config["MCP_servers"].as_table().unwrap();
    assert!(legacy["other"].is_table());
    assert!(legacy.contains_key("legacy"));
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
