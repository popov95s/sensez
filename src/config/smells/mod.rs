//! Per-language design-smell configuration.

mod defaults;
mod knobs;
mod raw;
mod resolve;
mod rules;
mod strictness;
mod validate;

pub use knobs::{Smells, Strictness};

use crate::spine::ir::Language;
use serde::Deserialize;

#[derive(Debug, Clone, Hash, Deserialize)]
#[serde(try_from = "raw::SmellsRaw")]
pub struct SmellConfig {
    pub enabled: bool,
    pub exclude: Vec<String>,
    python: Smells,
    javascript: Smells,
    typescript: Smells,
    rust: Smells,
}

impl SmellConfig {
    /// The resolved knob set for `lang`.
    pub fn for_language(&self, lang: Language) -> &Smells {
        match lang {
            Language::Python => &self.python,
            Language::JavaScript => &self.javascript,
            Language::TypeScript => &self.typescript,
            Language::Rust => &self.rust,
        }
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
            python: s.clone(),
            javascript: s.clone(),
            typescript: s.clone(),
            rust: s,
        }
    }
}

#[cfg(test)]
mod tests;
