use super::*;
use serde_json::json;

#[test]
fn defaults_to_quiet_changed_code_mode() {
    assert_eq!(
        Settings::from_lsp(&json!({})),
        Settings {
            health_enabled: true,
            ..Settings::default()
        }
    );
}

#[test]
fn accepts_editor_settings() {
    let value = json!({ "settings": { "sensez": { "diagnostics": { "level": "warning" }, "analysis": { "scope": "workspace" }, "repositoryHealth": { "enabled": false } } } });
    assert_eq!(
        Settings::from_lsp(&value),
        Settings {
            level: DisplayLevel::Warning,
            scope: AnalysisScope::Workspace,
            health_enabled: false
        }
    );
}
