use super::*;

#[test]
fn dispatch_uses_adapter_not_filename() {
    let tmp = tempfile::tempdir().unwrap();
    let misleading = tmp.path().join("opencode.json");
    McpAdapter::Standard
        .write(&misleading, "/bin/sensez")
        .unwrap();
    let config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(misleading).unwrap()).unwrap();
    assert!(config["mcpServers"]["sensez"].is_object());
    assert!(config["mcp"].is_null());
}

#[test]
fn unparseable_existing_config_is_an_error_not_a_reset() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join(".mcp.json");
    std::fs::write(&path, "{ not json }").unwrap();

    let err = McpAdapter::Standard
        .write(&path, "/bin/sensez")
        .unwrap_err();
    assert!(err.to_string().contains("refusing to modify"), "{err:#}");

    let after = std::fs::read_to_string(&path).unwrap();
    assert_eq!(after, "{ not json }", "original file must be untouched");
}

/// OpenCode's config is `.jsonc`; comments make strict JSON parsing fail,
/// which previously reset the whole file to just the sensez entry.
#[test]
fn jsonc_commented_config_is_refused_not_wiped() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("opencode.jsonc");
    std::fs::write(&path, "// my config\n{\"mcp\": {}}\n").unwrap();

    assert!(McpAdapter::OpenCode.write(&path, "/bin/sensez").is_err());
    let after = std::fs::read_to_string(&path).unwrap();
    assert!(after.contains("// my config"), "comments must survive");
}

/// `"mcpServers": []` used to panic the process via serde_json IndexMut.
#[test]
fn wrong_typed_keys_error_instead_of_panic() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join(".mcp.json");
    std::fs::write(&path, r#"{"mcpServers": []}"#).unwrap();

    let err = McpAdapter::Standard
        .write(&path, "/bin/sensez")
        .unwrap_err();
    assert!(err.to_string().contains("must be a JSON object"), "{err:#}");
}
