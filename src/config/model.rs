use super::brainz::SelfImprovement;
use super::smells::SmellConfig;
use crate::report::{ActionLevel, Severity, SmellKind};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Hash, Deserialize)]
#[serde(default)]
pub struct Config {
    pub roots: Vec<PathBuf>,
    pub exclude: Vec<String>,
    pub duplication: Duplication,
    pub dead_code: DeadCode,
    pub boundaries: Boundaries,
    pub smells: SmellConfig,
    pub action: ActionPolicy,
    pub gate: Gate,
    pub self_improvement: SelfImprovement,
    /// Team-shared accepted findings, keyed by pillar or detector id.
    pub accept: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Hash, Deserialize)]
#[serde(default)]
pub struct Duplication {
    pub exclude: Vec<String>,
    pub threshold: usize,
    pub max_gap: usize,
    pub near_miss: bool,
    pub class_name_duplicates: bool,
    pub class_property_overlap_min: usize,
}

#[derive(Debug, Clone, Hash, Deserialize)]
#[serde(default)]
pub struct DeadCode {
    pub entrypoints: Vec<String>,
    pub entrypoint_names: Vec<String>,
    pub entrypoint_bases: Vec<String>,
    pub entry_points: Vec<String>,
    #[serde(skip)]
    pub entry_modules: Vec<String>,
    pub unused_imports: bool,
    pub unused_methods: bool,
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

#[derive(Debug, Clone, Hash, Deserialize)]
#[serde(default)]
pub struct Gate {
    pub repeat_limit: usize,
}

impl ActionPolicy {
    pub fn for_smell(&self, kind: SmellKind, default_severity: Severity) -> ActionLevel {
        self.smells
            .get(&kind)
            .copied()
            .unwrap_or_else(|| ActionLevel::from_severity(default_severity))
    }
}

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
                class_name_duplicates: false,
                class_property_overlap_min: 4,
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
            gate: Gate::default(),
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

impl Default for Gate {
    fn default() -> Self {
        Gate { repeat_limit: 2 }
    }
}

impl Config {
    pub fn signature(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = rustc_hash::FxHasher::default();
        self.hash(&mut hasher);
        hasher.finish()
    }
}
