use super::*;

#[test]
fn earlier_gate_hook_formats_are_recognised() {
    for hook in [
        json!({"type": "mcp_tool", "tool": "noze_gate"}),
        json!({"type": "mcp_tool", "server": "sense", "tool": "gate"}),
        json!({"type": "command", "command": "npx sensez mcp gate"}),
    ] {
        assert!(is_gate_hook(&json!({"hooks": [hook]})), "{hook}");
    }
}

#[test]
fn unrelated_hooks_are_kept() {
    for hook in [
        json!({"type": "mcp_tool", "server": "other", "tool": "gate"}),
        json!({"type": "command", "command": "keep-me"}),
    ] {
        assert!(!is_gate_hook(&json!({"hooks": [hook]})), "{hook}");
    }
}

#[test]
fn reinstall_replaces_a_legacy_gate_instead_of_adding_one() {
    let mut settings = json!({"hooks": {"Stop": [
        {"hooks": [{"type": "mcp_tool", "server": "sense", "tool": "gate"}]}
    ]}});
    install(&mut settings, "Stop", gate_hook()).unwrap();
    let stop = settings["hooks"]["Stop"].as_array().unwrap();
    assert_eq!(stop.len(), 1);
    assert!(stop[0].to_string().contains("noze_gate"));
}
