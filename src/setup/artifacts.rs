use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;

const CONFIG_TEMPLATE: &str = r#"# sensez — the structural maintainability layer that complements your linter and
# type-checker (e.g. Ruff/ty for Python, ESLint/tsc for JS/TS): duplication,
# dead code, import cycles, boundary violations, and design smells. Everything
# sensez does is configured in this file; commented entries show the defaults.
# Re-run `sense init` anytime.

[self_improvement]
# sensez learns from each working session — which findings you fix, which you
# dismiss as false positives — to tune what it surfaces and to show its value
# (`brainz_report`). Stored ONLY under .sensez/local-metrics/ and NEVER transmitted
# anywhere regardless of this setting (see SECURITY.md). false stops even the
# on-disk recording.
enabled = true

[duplication]
# threshold = 50     # minimum matched token-run length to report a clone
# max_gap = 10       # Type-3 gapped-clone stitching (0 = off)
# near_miss = false  # consistent-rename clone detection (opt-in)

[dead_code]
# unused_imports = false
# unused_methods = false
# unused_variables = false
# entrypoints = []       # user-added decorator names; profile defaults are language-scoped
# entrypoint_names = []  # user-added dynamic function/class names
# entrypoint_bases = []  # user-added dynamic base classes
# entry_points = []      # user-added file globs reached outside the import graph

[action]
# Agent/gate policy for each pillar. Values: "info", "advisory", "warning", "must_fix".
# cycles = "warning"
# duplication = "advisory"
# dead_code = "advisory"
# boundaries = "must_fix"

[smells]
# enabled = true
# Per-rule thresholds; 0 disables a rule. The main knobs:
# max_cyclomatic, max_cognitive, max_function_lines, max_nesting,
# max_params, max_returns, max_class_methods, max_chain_depth
# param_attr_mutation = false  # also flag mutation through a param's attribute
#                              # (msg.kwargs[k] = v); stricter, opt-in

# Preferred per-smell shape: one table owns detection knobs + agent action.
# Setting a threshold/knob implicitly enables that smell; use enabled = false
# for explicit suppression.
# [smells.rules.split_variable]
# min_assigns = 3
# action = "info"
#
# [smells.rules.long_function]
# max_lines = 80
# action = "warning"

# Architectural contracts ("core must never import api"):
# [[boundaries.forbidden]]
# from = "core"
# to = "api"

# Accepted findings — the out-of-line alternative to `# noqa` (no annotations in
# your source). Committed + shared: every teammate's agent stops flagging these
# in the diff/gate loop. Key by pillar or detector; values are substrings of the
# finding label (as shown in brainz_report / a scan).
# [accept]
# dead_code = ["legacy.compat::shim"]          # a specific accepted symbol
# "smells/god_module" = ["app.registry"]       # a specific accepted detector hit
"#;

pub fn write_config(root: &Path, self_improvement: bool, into_pyproject: bool) -> Result<String> {
    let mut body = CONFIG_TEMPLATE.to_string();
    if !self_improvement {
        body = body.replace("\nenabled = true\n", "\nenabled = false\n");
    }
    if into_pyproject {
        let path = root.join("pyproject.toml");
        let existing = std::fs::read_to_string(&path).unwrap_or_default();
        if existing.contains("[tool.sensez") {
            return Ok("pyproject.toml already has [tool.sensez] — left as is".into());
        }
        let section: String = body
            .lines()
            .map(|line| {
                let prefixed = if let Some(rest) = line.strip_prefix("# [[") {
                    format!("# [[tool.sensez.{rest}")
                } else if let Some(rest) = line.strip_prefix('[') {
                    format!("[tool.sensez.{rest}")
                } else {
                    line.to_string()
                };
                prefixed + "\n"
            })
            .collect();
        std::fs::write(&path, format!("{existing}\n{section}"))
            .with_context(|| format!("appending {}", path.display()))?;
        Ok("added [tool.sensez] sections to pyproject.toml".into())
    } else {
        let path = root.join("sensez.toml");
        if path.exists() {
            return Ok("sensez.toml already exists — left as is".into());
        }
        std::fs::write(&path, body).with_context(|| format!("writing {}", path.display()))?;
        Ok("wrote sensez.toml with commented defaults".into())
    }
}

pub fn ensure_sensez_dir(root: &Path) -> Result<String> {
    let dir = root.join(".sensez");
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let ignore = dir.join(".gitignore");
    if !ignore.exists() {
        std::fs::write(&ignore, "*\n").with_context(|| format!("writing {}", ignore.display()))?;
    }
    Ok("created .sensez/ for local metrics and caches".into())
}

pub fn write_mcp_config(root: &Path, agent: &str, sensez_bin: &str) -> Result<String> {
    let rel = crate::setup::agents::find(agent)
        .and_then(|spec| spec.mcp_relpath)
        .ok_or_else(|| anyhow::anyhow!("no MCP config path is known for agent '{agent}'"))?;
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("toml") => write_mcp_toml(&path, sensez_bin)?,
        _ => write_mcp_json(&path, sensez_bin)?,
    }
    Ok(format!(
        "registered Sensez MCP server as `sense` in {}",
        path.display()
    ))
}

pub fn codex_mcp_add(sensez_bin: &str) -> Result<String> {
    let output = Command::new("codex")
        .args(["mcp", "add", "sense", sensez_bin, "mcp", "serve"])
        .output()
        .context("running `codex mcp add`")?;
    if output.status.success() {
        return Ok("registered Sensez MCP server as `sense` with `codex mcp add`".into());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(anyhow::anyhow!(
        "`codex mcp add` failed (status {}): {}",
        output.status,
        stderr.trim()
    ))
}

fn write_mcp_json(path: &Path, sensez_bin: &str) -> Result<()> {
    let mut config: Value = std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_else(|| json!({}));
    config["mcpServers"]["sense"] = json!({"command": sensez_bin, "args": ["mcp", "serve"]});
    std::fs::write(path, serde_json::to_string_pretty(&config)?)
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

fn write_mcp_toml(path: &Path, sensez_bin: &str) -> Result<()> {
    let mut config: toml::Value = std::fs::read_to_string(path)
        .ok()
        .and_then(|t| toml::from_str(&t).ok())
        .unwrap_or_else(|| toml::Value::Table(toml::map::Map::new()));
    let table = config
        .as_table_mut()
        .ok_or_else(|| anyhow::anyhow!("{} must be a TOML table", path.display()))?;
    let mcp = table
        .entry("MCP_servers")
        .or_insert_with(|| toml::Value::Table(toml::map::Map::new()))
        .as_table_mut()
        .ok_or_else(|| anyhow::anyhow!("MCP_servers must be a TOML table"))?;
    let mut sense = toml::map::Map::new();
    sense.insert(
        "command".to_string(),
        toml::Value::String(sensez_bin.to_string()),
    );
    sense.insert(
        "args".to_string(),
        toml::Value::Array(vec![
            toml::Value::String("mcp".to_string()),
            toml::Value::String("serve".to_string()),
        ]),
    );
    mcp.insert("sense".to_string(), toml::Value::Table(sense));
    std::fs::write(path, toml::to_string_pretty(&config)?)
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

pub fn write_gate(root: &Path) -> Result<String> {
    let settings_path = root.join(".claude/settings.json");
    if let Some(parent) = settings_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let mut settings: Value = std::fs::read_to_string(&settings_path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_else(|| json!({}));
    let mut stops = settings["hooks"]["Stop"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if stops.iter().any(|e| {
        let text = e.to_string();
        text.contains("\"tool\":\"noze_gate\"") || text.contains("\"tool\":\"gate\"")
    }) {
        return Ok("Stop-gate already installed (mcp_tool -> sense noze_gate)".into());
    }
    stops.push(json!({"hooks": [{
        "type": "mcp_tool",
        "server": "sense",
        "tool": "noze_gate",
        "input": {"path": "${cwd}", "stop_hook_active": "${stop_hook_active}"},
        "timeout": 60,
        "statusMessage": "sense: scanning session changes"
    }]}));
    settings["hooks"]["Stop"] = json!(stops);
    std::fs::write(&settings_path, serde_json::to_string_pretty(&settings)?)
        .with_context(|| format!("writing {}", settings_path.display()))?;
    Ok("installed Stop-gate hook (mcp_tool -> sense noze_gate, shared session)".into())
}

pub fn ensure_gitignore(root: &Path) -> Result<String> {
    let path = root.join(".gitignore");
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    if existing
        .lines()
        .any(|l| l.trim() == ".sensez/" || l.trim() == ".sensez")
    {
        return Ok(".gitignore already covers .sensez/".into());
    }
    let mut next = existing;
    if !next.is_empty() && !next.ends_with('\n') {
        next.push('\n');
    }
    next.push_str("\n# sensez local data (eyez cache + local-only metrics)\n.sensez/\n");
    std::fs::write(&path, next).with_context(|| format!("writing {}", path.display()))?;
    Ok("added .sensez/ to .gitignore".into())
}
