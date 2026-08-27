use anyhow::{Context, Result};
use std::path::Path;

fn read_existing(path: &Path) -> Result<String> {
    match std::fs::read(path) {
        Ok(bytes) => String::from_utf8(bytes)
            .map_err(|_| anyhow::anyhow!("{} is not valid UTF-8", path.display())),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(err) => Err(anyhow::Error::new(err).context(format!("reading {}", path.display()))),
    }
}

const CONFIG_TEMPLATE: &str = r#"# sensez — the structural maintainability layer that complements your linter and
# type-checker (e.g. Ruff/ty for Python, ESLint/tsc for JS/TS): duplication,
# dead code, import cycles, boundary violations, and design smells. Everything
# sensez does is configured in this file; commented entries show the defaults.
# Re-run `sensez init` anytime.

[cache]
# CLI scans are stateless. Long-lived MCP/LSP servers reuse process-local
# analysis state and never persist parsed source facts under .sensez/.
# Set enabled = true to enable this reuse for MCP/LSP. It has no effect on CLI.
# enabled = false

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
# class_name_duplicates = false  # same-name class duplication. Can be noisy for teams that follow same patterns for business logic services, stores, etc.(opt-in)
# class_property_overlap_min = 4  # same-name/same-type class fields (0 = off)
#
# [duplication.semantic]  # opt-in, requires the `eyez` feature
# enabled = false
# min_shape_score = 82       # structural candidate gate, as a 0-100 score
# comment_boost_score = 85   # doc/comment embedding score needed to promote
# comment_required = true    # default: semantic findings need comments/docstrings

[dead_code]
# unused_imports = false
# unused_methods = false
# unused_properties = true
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
        let existing = read_existing(&path)?;
        if existing.contains("[tool.sensez") {
            return Ok("pyproject.toml already has [tool.sensez] — left as is".into());
        }
        if !existing.trim().is_empty() {
            existing.parse::<toml::Value>().with_context(|| {
                format!(
                    "validating {} before appending [tool.sensez] — refusing to modify",
                    path.display()
                )
            })?;
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
    let spec = crate::setup::agents::find(agent)
        .ok_or_else(|| anyhow::anyhow!("unknown agent '{agent}'"))?;
    let rel = spec
        .mcp_relpath
        .ok_or_else(|| anyhow::anyhow!("no MCP config path is known for agent '{agent}'"))?;
    write_mcp_config_at(&root.join(rel), spec, sensez_bin)
}

pub fn write_global_mcp_config(root: &Path, agent: &str, sensez_bin: &str) -> Result<String> {
    let spec = crate::setup::agents::find(agent)
        .ok_or_else(|| anyhow::anyhow!("unknown agent '{agent}'"))?;
    let rel = spec
        .global_mcp_relpath
        .ok_or_else(|| anyhow::anyhow!("no global MCP config path is known for agent '{agent}'"))?;
    write_mcp_config_at(&root.join(rel), spec, sensez_bin)
}

fn write_mcp_config_at(
    path: &Path,
    spec: &crate::setup::agents::AgentSpec,
    sensez_bin: &str,
) -> Result<String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let adapter = spec
        .mcp_adapter
        .ok_or_else(|| anyhow::anyhow!("no MCP adapter is known for agent '{}'", spec.id))?;
    adapter.write(path, sensez_bin)?;
    Ok(format!(
        "registered Sensez MCP server as `sensez` in {}",
        path.display()
    ))
}

pub fn write_gate(root: &Path) -> Result<String> {
    super::gate::write(root)
}

pub fn ensure_gitignore(root: &Path) -> Result<String> {
    let path = root.join(".gitignore");
    let existing = read_existing(&path)?;
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
