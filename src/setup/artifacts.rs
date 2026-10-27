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
# threshold = 50     # minimum matched token-run length to report a clone (0 = off)
# max_gap = 25       # Type-3 gapped-clone stitching (0 = off)
# near_miss = false  # true also compares renamed copies shorter than twice the threshold
# class_name_duplicates = false  # same-name class duplication. Can be noisy for teams that follow same patterns for business logic services, stores, etc.(opt-in)
# class_property_overlap_min = 4  # same-name/same-type class fields (0 = off)
# include_tests = false      # true also compares test files (profile test globs, tests/, test/)
# include_generated = false  # true also compares generated files (_generated/, *.gen.*, @generated headers)
# exclude = ["**/fixtures/**"]  # extra clone-only excludes (files stay in the graph)
#
# [duplication.semantic]  # opt-in, requires the `eyez` feature
# enabled = false
# min_shape_score = 30       # token 4-gram overlap (0-100) separating edited copies from rewrites
# comment_boost_score = 85   # doc similarity (50-100) that confirms an edited copy
# comment_required = false   # true compares documented functions only

[dead_code]
# unused_imports = false
# unused_methods = false
# unused_properties = false
# unused_variables = false
# registration_decorators = []  # user-added decorator names (JS/TS: `export const x = name(...)` callees); `prefix*` allowed
# entrypoint_names = []  # user-added dynamic function/class/method names; `prefix*` allowed
# entrypoint_bases = []  # user-added dynamic base classes
# entry_globs = []      # user-added file globs reached outside the import graph (roots)
# test_sources = []      # user-added test-runner file globs (test roots)
# test_support = []      # user-added test-helper globs: test importers count as users
# orphan_modules = true  # report modules no root reaches (Medium, "unreachable_module")
# unimported_dir_share = 50  # % of a directory's modules nothing imports that marks it runtime-loaded (0 = off)

[cycles]
# include_type_only = true  # false hides cycles closed only by type imports; recommended for TypeScript, where they vanish at build time

[action]
# Agent/gate policy for each pillar. Values: "info", "advisory", "warning", "must_fix".
# cycles = "warning"
# duplication = "advisory"
# dead_code = "advisory"
# boundaries = "must_fix"

[smells]
# enabled = true
# include_tests = false  # true also runs smells on test files (profile test globs, tests/, test/)
#
# One [smells.rules.<name>] table per smell holds its detection knobs and its
# agent action. Setting any knob, including `action`, enables the rule, and
# `enabled = false` turns it off.
# [smells.rules.long_function]
# max_lines = 50
# action = "warning"
#
# [smells.rules.high_complexity]
# max_cyclomatic = 10
#
# [smells.rules.split_variable]
# min_assigns = 2
# action = "info"
#
# [smells.rules.mutated_parameter]
# include_attributes = false  # also flag mutation through a param's attribute
#                             # (msg.kwargs[k] = v); stricter, opt-in
#
# [smells.rules.too_many_returns]
# count_guard_clauses = false  # also count top-level guard-clause returns
#
# [smells.rules.magic_numbers]
# ignore = []                   # numeric literals never reported
# ignore_array_indexes = true   # skip literal array indexes such as x[3]
# ignore_default_values = true  # skip literals in parameter default values
# allow_http_status = true      # skip 100-599 compared or assigned to a status field

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
# duplication = ["legacy-fork/"]               # drop this location from every clone group; "a.ts + b.ts" hides one whole group
# "smells/god_module" = ["app.registry"]       # a specific accepted detector hit
"#;

pub struct WriteConfigOptions {
    pub self_improvement: bool,
    pub into_pyproject: bool,
    pub semantic_duplication: bool,
}

/// Whether `init` wrote a fresh config or kept the one already there.
pub enum ConfigWrite {
    Created(String),
    Kept(String),
}

impl ConfigWrite {
    pub fn message(&self) -> &str {
        match self {
            ConfigWrite::Created(message) | ConfigWrite::Kept(message) => message,
        }
    }
}

pub fn write_config(root: &Path, options: WriteConfigOptions) -> Result<ConfigWrite> {
    let mut body = CONFIG_TEMPLATE.to_string();
    if !options.self_improvement {
        body = body.replace("\nenabled = true\n", "\nenabled = false\n");
    }
    if options.semantic_duplication {
        body.push_str(
            "\n[duplication.semantic]\n\
             enabled = true\n\
             # min_shape_score = 30\n\
             # comment_boost_score = 85\n\
             # comment_required = false\n",
        );
    }
    if options.into_pyproject {
        let path = root.join("pyproject.toml");
        let existing = read_existing(&path)?;
        if existing.contains("[tool.sensez") {
            return Ok(ConfigWrite::Kept(
                "pyproject.toml already has [tool.sensez] — left as is".into(),
            ));
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
        Ok(ConfigWrite::Created(
            "added [tool.sensez] sections to pyproject.toml".into(),
        ))
    } else {
        let path = root.join("sensez.toml");
        if path.exists() {
            return Ok(ConfigWrite::Kept(
                "sensez.toml already exists — left as is".into(),
            ));
        }
        std::fs::write(&path, body).with_context(|| format!("writing {}", path.display()))?;
        Ok(ConfigWrite::Created(
            "wrote sensez.toml with commented defaults".into(),
        ))
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
