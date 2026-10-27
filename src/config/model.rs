use super::brainz::SelfImprovement;
use super::smells::SmellConfig;
use crate::report::{ActionLevel, Severity, SmellKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub cache: Cache,
    pub roots: Vec<PathBuf>,
    pub exclude: Vec<String>,
    pub duplication: Duplication,
    pub dead_code: DeadCode,
    pub cycles: Cycles,
    pub boundaries: Boundaries,
    pub smells: SmellConfig,
    pub action: ActionPolicy,
    pub gate: Gate,
    pub self_improvement: SelfImprovement,
    /// Team-shared accepted findings, keyed by pillar or detector id.
    pub accept: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Default, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct Cache {
    pub enabled: bool,
}

#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct Duplication {
    pub exclude: Vec<String>,
    pub include_tests: bool,
    pub include_generated: bool,
    pub threshold: usize,
    pub max_gap: usize,
    pub near_miss: bool,
    pub semantic: SemanticDuplication,
    pub class_name_duplicates: bool,
    pub class_property_overlap_min: usize,
}

#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct SemanticDuplication {
    pub enabled: bool,
    pub min_shape_score: u8,
    pub comment_boost_score: u8,
    pub comment_required: bool,
}

#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct DeadCode {
    /// Decorator (JS/TS: initializer callee) names that register a symbol, keeping it live.
    #[serde(alias = "entrypoints")]
    pub registration_decorators: Vec<String>,
    pub entrypoint_names: Vec<String>,
    pub entrypoint_bases: Vec<String>,
    /// File globs whose modules are reached outside the import graph.
    #[serde(alias = "entry_points")]
    pub entry_globs: Vec<String>,
    pub test_sources: Vec<String>,
    pub test_support: Vec<String>,
    #[serde(skip)]
    pub entry_modules: Vec<String>,
    pub orphan_modules: bool,
    /// Percent of a directory's modules with no importers that marks it as runtime-loaded (0 = off).
    pub unimported_dir_share: u8,
    pub unused_imports: bool,
    pub unused_methods: bool,
    pub unused_properties: bool,
    pub unused_variables: bool,
}

#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct Cycles {
    pub exclude: Vec<String>,
    /// Also report cycles closed only by type-level imports (at `info`).
    pub include_type_only: bool,
}

#[derive(Debug, Clone, Default, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct Boundaries {
    pub forbidden: Vec<ForbiddenRule>,
}

#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct ActionPolicy {
    pub cycles: ActionLevel,
    pub duplication: ActionLevel,
    pub dead_code: ActionLevel,
    pub boundaries: ActionLevel,
    pub smells: BTreeMap<SmellKind, ActionLevel>,
}

#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
pub struct ForbiddenRule {
    pub from: String,
    pub to: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            cache: Cache::default(),
            roots: Vec::new(),
            exclude: Vec::new(),
            duplication: Duplication {
                exclude: Vec::new(),
                include_tests: false,
                include_generated: false,
                threshold: 50,
                max_gap: 25,
                near_miss: false,
                semantic: SemanticDuplication::default(),
                class_name_duplicates: false,
                class_property_overlap_min: 4,
            },
            dead_code: DeadCode {
                registration_decorators: Vec::new(),
                entrypoint_names: Vec::new(),
                entrypoint_bases: Vec::new(),
                entry_globs: Vec::new(),
                test_sources: Vec::new(),
                test_support: Vec::new(),
                entry_modules: Vec::new(),
                orphan_modules: true,
                unimported_dir_share: 50,
                unused_imports: false,
                unused_methods: false,
                unused_properties: false,
                unused_variables: false,
            },
            boundaries: Boundaries {
                forbidden: Vec::new(),
            },
            cycles: Cycles::default(),
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

impl Default for SemanticDuplication {
    fn default() -> Self {
        SemanticDuplication {
            enabled: false,
            min_shape_score: 30,
            comment_boost_score: 85,
            comment_required: false,
        }
    }
}

impl Default for DeadCode {
    fn default() -> Self {
        Config::default().dead_code
    }
}

impl Default for Cycles {
    fn default() -> Self {
        Cycles {
            exclude: Vec::new(),
            include_type_only: true,
        }
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
    pub fn cache_enabled(&self) -> bool {
        match std::env::var("SENSEZ_ANALYSIS_CACHE") {
            Ok(value)
                if matches!(
                    value.to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                ) =>
            {
                true
            }
            Ok(value)
                if matches!(
                    value.to_ascii_lowercase().as_str(),
                    "0" | "false" | "no" | "off"
                ) =>
            {
                false
            }
            _ => self.cache.enabled,
        }
    }

    pub fn signature(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = rustc_hash::FxHasher::default();
        self.hash(&mut hasher);
        hasher.finish()
    }
}
