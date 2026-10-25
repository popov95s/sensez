//! Knob values compiled once per `Smells` on first use.

use super::knobs::Smells;
use crate::globs::compile_validated;
use globset::GlobSet;
use std::hash::{Hash, Hasher};
use std::sync::OnceLock;

/// Caches derived from knob fields; hashes as nothing since the fields already do.
#[derive(Debug, Clone, Default)]
pub struct Compiled {
    assertion_functions: OnceLock<GlobSet>,
    magic_number_ignore: OnceLock<Vec<f64>>,
}

impl Hash for Compiled {
    fn hash<H: Hasher>(&self, _state: &mut H) {}
}

impl Smells {
    /// `assertion_functions` as one glob set.
    pub fn assertion_globs(&self) -> &GlobSet {
        self.compiled
            .assertion_functions
            .get_or_init(|| compile_validated(&self.assertion_functions))
    }

    /// `magic_number_ignore` parsed as numbers.
    pub fn ignored_magic_numbers(&self) -> &[f64] {
        self.compiled.magic_number_ignore.get_or_init(|| {
            self.magic_number_ignore
                .iter()
                .filter_map(|text| text.parse().ok())
                .collect()
        })
    }
}
