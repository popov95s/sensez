#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentKind {
    ClaudeCode,
    Cursor,
    Cline,
    Codex,
    OpenCode,
    Pi,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AgentSpec {
    pub kind: AgentKind,
    pub id: &'static str,
    pub label: &'static str,
    pub mcp_relpath: Option<&'static str>,
    pub skill_relpath: Option<&'static str>,
    pub supports_hooks: bool,
}

pub const AGENTS: &[AgentSpec] = &[
    AgentSpec {
        kind: AgentKind::ClaudeCode,
        id: "claude-code",
        label: "Claude Code",
        mcp_relpath: Some(".mcp.json"),
        skill_relpath: Some(".claude/skills/sensez"),
        supports_hooks: true,
    },
    AgentSpec {
        kind: AgentKind::Cursor,
        id: "cursor",
        label: "Cursor",
        mcp_relpath: Some(".cursor/mcp.json"),
        skill_relpath: None,
        supports_hooks: false,
    },
    AgentSpec {
        kind: AgentKind::Cline,
        id: "cline",
        label: "Cline",
        mcp_relpath: Some(".cline/mcp.json"),
        skill_relpath: None,
        supports_hooks: false,
    },
    AgentSpec {
        kind: AgentKind::Codex,
        id: "codex",
        label: "Codex",
        mcp_relpath: Some(".codex/config.toml"),
        skill_relpath: Some(".codex/skills/sensez"),
        supports_hooks: false,
    },
    AgentSpec {
        kind: AgentKind::OpenCode,
        id: "opencode",
        label: "OpenCode",
        mcp_relpath: Some("opencode.jsonc"),
        skill_relpath: Some(".opencode/skills/sensez"),
        supports_hooks: false,
    },
    AgentSpec {
        kind: AgentKind::Pi,
        id: "pi",
        label: "Pi",
        mcp_relpath: Some(".pi/mcp.json"),
        skill_relpath: None,
        supports_hooks: false,
    },
    AgentSpec {
        kind: AgentKind::Other,
        id: "other",
        label: "Other / none",
        mcp_relpath: None,
        skill_relpath: None,
        supports_hooks: false,
    },
];

pub const INTERACTIVE_AGENTS: &[AgentSpec] = &[
    AGENTS[0], AGENTS[1], AGENTS[2], AGENTS[3], AGENTS[4], AGENTS[5], AGENTS[6],
];

pub fn find(id: &str) -> Option<&'static AgentSpec> {
    AGENTS.iter().find(|agent| agent.id == id)
}

pub fn from_choice(choice: usize) -> &'static AgentSpec {
    &AGENTS[choice]
}

pub fn prompt_options() -> Vec<&'static str> {
    INTERACTIVE_AGENTS.iter().map(|agent| agent.label).collect()
}
