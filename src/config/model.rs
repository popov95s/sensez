use super::brainz::SelfImprovement;
use super::smells::SmellConfig;
use crate::report::{ActionLevel, Severity, SmellKind};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Top-level configuration loaded from `sensez.toml`.
#[derive(Debug, Clone, Hash, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Package-root overrides (relative to the project root). Empty = auto.
    pub roots: Vec<PathBuf>,
    /// File globs excluded from the scan entirely (affects every pillar).
    /// e.g. `["**/tests/**", "**/migrations/**"]`.
    pub exclude: Vec<String>,
    pub duplication: Duplication,
    pub dead_code: DeadCode,
    pub boundaries: Boundaries,
    pub smells: SmellConfig,
    /// Finding action levels for agents/gates. Detector thresholds decide what
    /// exists; this policy decides whether a finding is informational,
    /// advisory, warning-level, or must-fix.
    pub action: ActionPolicy,
    /// Local-only self-improvement data opt-out (`[self_improvement]`). Never
    /// streamed; drives `brainz_report` and triage-based suppression.
    pub self_improvement: SelfImprovement,
    /// Committed, team-shared "accepted findings" — the out-of-line alternative
    /// to `# noqa` (no source annotations). `[accept]` maps a pillar
    /// (`dead_code`) or detector (`smells/god_module`) to label substrings; a
    /// finding whose label contains one is suppressed in the diff/gate loop for
    /// everyone on the repo. Unlike personal triage (gitignored `.sensez/`), this
    /// lives in the config and is version-controlled.
    pub accept: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Hash, Deserialize)]
#[serde(default)]
pub struct Duplication {
    /// File globs excluded from duplication only (kept in the graph for the
    /// other pillars). Defaults to tests/migrations/alembic.
    pub exclude: Vec<String>,
    /// Minimum matched token-run length to report a clone.
    pub threshold: usize,
    /// Type-3 (gapped) clones: stitch two clones separated by at most this many
    /// tokens when the gap is consistent across occurrences. 0 disables.
    pub max_gap: usize,
    /// Opt-in (default false): also report consistent-rename near-miss clones —
    /// functions identical up to a 1:1 renaming of names/types/literals (the
    /// default matcher stays strict and only collapses function-local vars).
    pub near_miss: bool,
}

#[derive(Debug, Clone, Hash, Deserialize)]
#[serde(default)]
pub struct DeadCode {
    /// Decorator names that mark framework entrypoints (never dead).
    pub entrypoints: Vec<String>,
    /// Function/class names that are entrypoints by convention — invoked
    /// dynamically by frameworks/plugin loaders (e.g. `register`, `main`).
    pub entrypoint_names: Vec<String>,
    /// Base-class names whose subclasses are discovered dynamically by a
    /// framework, plugin loader, or runtime manifest. Applies across languages
    /// that expose class bases in the shared IR.
    pub entrypoint_bases: Vec<String>,
    /// File globs whose modules are entry points reached outside the import
    /// graph (path-discovered runners: alembic, custom loaders). Their symbols
    /// are never flagged. e.g. `["**/migrations/**", "**/versions/**"]`.
    pub entry_points: Vec<String>,
    /// Module names derived from `pyproject.toml` entry points at runtime
    /// (console scripts, plugin entry points). Not read from `sensez.toml`.
    #[serde(skip)]
    pub entry_modules: Vec<String>,
    /// Opt-in: report unused imports (off by default).
    pub unused_imports: bool,
    /// Opt-in: report unused methods — class-level functions never referenced
    /// in their own module (off by default; cross-file attribute access is
    /// invisible, so these are Low confidence).
    pub unused_methods: bool,
    /// Opt-in: report unused module-level variables (off by default).
    pub unused_variables: bool,
}

#[derive(Debug, Clone, Default, Hash, Deserialize)]
#[serde(default)]
pub struct Boundaries {
    pub forbidden: Vec<ForbiddenRule>,
}

#[derive(Debug, Clone, Hash, Deserialize)]
#[serde(default)]
pub struct ActionPolicy {
    pub cycles: ActionLevel,
    pub duplication: ActionLevel,
    pub dead_code: ActionLevel,
    pub boundaries: ActionLevel,
    pub smells: BTreeMap<SmellKind, ActionLevel>,
}

impl ActionPolicy {
    pub fn for_smell(&self, kind: SmellKind, default_severity: Severity) -> ActionLevel {
        self.smells
            .get(&kind)
            .copied()
            .unwrap_or_else(|| ActionLevel::from_severity(default_severity))
    }
}

/// A forbidden import edge: any module under `from` may not import `to`.
#[derive(Debug, Clone, Hash, Deserialize)]
pub struct ForbiddenRule {
    pub from: String,
    pub to: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            roots: Vec::new(),
            exclude: Vec::new(),
            duplication: Duplication {
                exclude: super::BASELINE_EXCLUDE
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                threshold: 50,
                max_gap: 10,
                near_miss: false,
            },
            dead_code: DeadCode {
                entrypoints: Vec::new(),
                entrypoint_names: Vec::new(),
                entrypoint_bases: Vec::new(),
                entry_points: Vec::new(),
                entry_modules: Vec::new(),
                unused_imports: false,
                unused_methods: false,
                unused_variables: false,
            },
            boundaries: Boundaries {
                forbidden: Vec::new(),
            },
            smells: SmellConfig::default(),
            action: ActionPolicy::default(),
            self_improvement: SelfImprovement::default(),
            accept: BTreeMap::new(),
        }
    }
}

impl Default for Duplication {
    fn default() -> Self {
        Config::default().duplication
    }
}

impl Default for DeadCode {
    fn default() -> Self {
        Config::default().dead_code
    }
}

impl Default for ActionPolicy {
    fn default() -> Self {
        ActionPolicy {
            cycles: ActionLevel::Warning,
            duplication: ActionLevel::Advisory,
            dead_code: ActionLevel::Advisory,
            boundaries: ActionLevel::MustFix,
            smells: BTreeMap::new(),
        }
    }
}

impl Config {
    /// Stable hash of the effective configuration, used to detect config churn
    /// between scans.
    pub fn signature(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = rustc_hash::FxHasher::default();
        self.hash(&mut hasher);
        hasher.finish()
    }
}
