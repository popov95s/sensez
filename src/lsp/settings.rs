use serde_json::Value;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DisplayLevel {
    #[default]
    MustFix,
    Warning,
    Advisory,
    Info,
    Off,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AnalysisScope {
    #[default]
    Changed,
    Workspace,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Settings {
    pub level: DisplayLevel,
    pub scope: AnalysisScope,
    pub health_enabled: bool,
}

impl Settings {
    pub fn from_lsp(value: &Value) -> Self {
        let sensez = value
            .pointer("/settings/sensez")
            .or_else(|| value.pointer("/initializationOptions/sensez"))
            .or_else(|| value.get("sensez"));
        let Some(sensez) = sensez else {
            return Self {
                health_enabled: true,
                ..Self::default()
            };
        };
        Self {
            level: display_level(sensez.pointer("/diagnostics/level")),
            scope: scope(sensez.pointer("/analysis/scope")),
            health_enabled: sensez
                .pointer("/repositoryHealth/enabled")
                .and_then(Value::as_bool)
                .unwrap_or(true),
        }
    }
}

fn display_level(value: Option<&Value>) -> DisplayLevel {
    match value.and_then(Value::as_str) {
        Some("warning") => DisplayLevel::Warning,
        Some("advisory") => DisplayLevel::Advisory,
        Some("info") => DisplayLevel::Info,
        Some("off") => DisplayLevel::Off,
        _ => DisplayLevel::MustFix,
    }
}

fn scope(value: Option<&Value>) -> AnalysisScope {
    match value.and_then(Value::as_str) {
        Some("workspace") => AnalysisScope::Workspace,
        _ => AnalysisScope::Changed,
    }
}

#[cfg(test)]
mod tests {
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
}
