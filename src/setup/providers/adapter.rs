use super::{codex, opencode, standard};
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
