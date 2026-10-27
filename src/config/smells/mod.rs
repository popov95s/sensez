//! Per-language design-smell configuration.

mod compiled;
mod defaults;
mod knob_table;
mod knobs;
mod raw;
mod resolve;
mod rule_lists;
mod rules;
mod strictness;
mod toggles;

pub use knobs::{Smells, Strictness};
pub(crate) use rules::rule_keys;

use crate::spine::ir::Language;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
#[serde(try_from = "raw::SmellsRaw")]
pub struct SmellConfig {
    pub enabled: bool,
    pub exclude: Vec<String>,
    /// User-supplied exclusions, without the automatic baseline patterns.
    /// Opt-in test analysis still honors these paths.
    #[serde(skip)]
    pub explicit_exclude: Vec<String>,
    pub include_tests: bool,
    #[serde(flatten)]
    languages: LanguageSmells,
}

/// One resolved knob set per language, serialized under each language's config key.
#[derive(Debug, Clone, Hash)]
struct LanguageSmells(BTreeMap<Language, Smells>);

impl Serialize for LanguageSmells {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_map(self.0.iter().map(|(lang, smells)| (lang.key(), smells)))
    }
}

impl SmellConfig {
    /// The resolved knob set for `lang`.
    pub fn for_language(&self, lang: Language) -> &Smells {
        &self.languages.0[&lang]
    }
}

impl Default for SmellConfig {
    fn default() -> Self {
        match raw::SmellsRaw::default().try_into() {
            Ok(config) => config,
            Err(err) => panic!("built-in smell defaults are invalid: {err}"),
        }
    }
}

impl TryFrom<raw::SmellsRaw> for SmellConfig {
    type Error = String;

    fn try_from(raw: raw::SmellsRaw) -> Result<Self, Self::Error> {
        resolve::resolve_config(raw)
    }
}

impl From<Smells> for SmellConfig {
    fn from(s: Smells) -> Self {
        SmellConfig {
            enabled: s.enabled,
            exclude: s.exclude.clone(),
            explicit_exclude: s.exclude.clone(),
            include_tests: false,
            languages: LanguageSmells(Language::ALL.map(|lang| (lang, s.clone())).into()),
        }
    }
}

#[cfg(test)]
mod flat_knob_tests;
#[cfg(test)]
mod knob_type_tests;
#[cfg(test)]
mod noise_rule_tests;
#[cfg(test)]
mod serde_tests;
#[cfg(test)]
mod tests;
