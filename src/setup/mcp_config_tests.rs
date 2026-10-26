//! MCP config writers keep existing content.

use super::tests::temp_root;
use super::*;
use std::fs;

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
fn init_preserves_mcp_json_key_order_and_newline() {
    let (_tmp, root) = temp_root();
    fs::write(root.join(".mcp.json"), "{\"zeta\":1,\"mcpServers\":{}}\n").unwrap();

    artifacts::write_mcp_config(&root, "claude-code", "/bin/sensez").unwrap();

    let text = fs::read_to_string(root.join(".mcp.json")).unwrap();
    assert!(text.find("zeta") < text.find("mcpServers"), "{text}");
    assert!(text.find("\"command\"") < text.find("\"args\""), "{text}");
    assert!(text.ends_with("}\n"), "{text:?}");
}
