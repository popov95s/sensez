use super::*;
use std::fs;

#[test]
fn global_paths_match_each_agent_convention() {
    assert_eq!(
        agents::find("cline").unwrap().global_mcp_relpath,
        Some(".cline/data/settings/cline_mcp_settings.json")
    );
    assert_eq!(
        agents::find("opencode").unwrap().global_mcp_relpath,
        Some(".config/opencode/opencode.json")
    );
    assert_eq!(
        agents::find("pi").unwrap().global_skill_relpath,
        Some(".pi/agent/skills/sensez")
    );
}

#[test]
fn non_repository_yes_installs_each_agent_globally() {
    for agent in agents::AGENTS
        .iter()
        .filter(|agent| agent.global_mcp_relpath.is_some())
    {
        let project = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let project_root = project.path().canonicalize().unwrap();
        let home_root = home.path().canonicalize().unwrap();

        run_with_home(
            InitOptions {
                path: Some(project_root.clone()),
                agent: Some(agent.id.to_string()),
                gate: false,
                no_metrics: false,
                yes: true,
            },
            Some(home_root.clone()),
        )
        .unwrap();
        run_with_home(
            InitOptions {
                path: Some(project_root.clone()),
                agent: Some(agent.id.to_string()),
                gate: false,
                no_metrics: false,
                yes: true,
            },
            Some(home_root.clone()),
        )
        .unwrap();

        let mcp_path = home_root.join(agent.global_mcp_relpath.unwrap());
        assert!(
            mcp_path.exists(),
            "missing global MCP config for {}",
            agent.id
        );
        match agent.kind {
            agents::AgentKind::Codex => {
                let config: toml::Value =
                    toml::from_str(&fs::read_to_string(mcp_path).unwrap()).unwrap();
                assert!(config["mcp_servers"]["sensez"].is_table());
            }
            agents::AgentKind::OpenCode => {
                let config: serde_json::Value =
                    serde_json::from_str(&fs::read_to_string(mcp_path).unwrap()).unwrap();
                assert!(config["mcp"]["sensez"].is_object());
            }
            _ => {
                let config: serde_json::Value =
                    serde_json::from_str(&fs::read_to_string(mcp_path).unwrap()).unwrap();
                assert!(config["mcpServers"]["sensez"].is_object());
            }
        }
        if let Some(skill) = agent.global_skill_relpath {
            assert!(
                home_root.join(skill).join("SKILL.md").exists(),
                "missing global skill for {}",
                agent.id
            );
        }
        assert!(!project_root.join("sensez.toml").exists());
        assert!(!project_root.join(".sensez").exists());
        assert!(!project_root.join(".gitignore").exists());
    }
}

#[test]
fn global_claude_gate_uses_user_settings() {
    let project = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let home_root = home.path().canonicalize().unwrap();
    run_with_home(
        InitOptions {
            path: Some(project.path().canonicalize().unwrap()),
            agent: Some("claude-code".to_string()),
            gate: true,
            no_metrics: false,
            yes: true,
        },
        Some(home_root.clone()),
    )
    .unwrap();
    assert!(home_root.join(".claude/settings.json").exists());
}

#[test]
fn nested_directory_is_still_a_project_install() {
    let repo = tempfile::tempdir().unwrap();
    let root = repo.path().canonicalize().unwrap();
    fs::create_dir(root.join(".git")).unwrap();
    let nested = root.join("nested");
    fs::create_dir(&nested).unwrap();
    let target = scope::Target::resolve(Some(&nested)).unwrap();
    assert_eq!(target.repository_root, Some(root));
}

#[test]
fn no_tty_no_flags_refuses_instead_of_hanging() {
    let repo = tempfile::tempdir().unwrap();
    let root = repo.path().canonicalize().unwrap();
    fs::create_dir(root.join(".git")).unwrap();
    let result = run(InitOptions {
        path: Some(root),
        agent: None,
        gate: false,
        no_metrics: false,
        yes: false,
    });
    assert!(result.is_err());
}
