mod codex;
mod opencode;
mod standard;

use anyhow::Result;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum McpAdapter {
    Standard,
    Codex,
    OpenCode,
}

impl McpAdapter {
    pub fn write(self, path: &Path, sensez_bin: &str) -> Result<()> {
        match self {
            Self::Standard => standard::write(path, sensez_bin),
            Self::Codex => codex::write(path, sensez_bin),
            Self::OpenCode => opencode::write(path, sensez_bin),
        }
    }
}

#[cfg(test)]
mod tests {
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
}
