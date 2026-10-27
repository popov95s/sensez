//! Built-in per-language smell defaults.
//!
//! Python and Rust use the baseline [`Smells::default`]. JavaScript/TypeScript defer the rules
//! ESLint / typescript-eslint / SonarJS own and keep the heuristic, cross-file,
//! and type-discipline smells. Each profile may switch off more smells on top.

use super::knobs::Smells;
use crate::profiles::registry;
use crate::report::SmellKind;
use crate::spine::ir::Language;

/// The built-in default knob set for `lang`, before any `sensez.toml` overlay.
pub fn default_for(lang: Language) -> Smells {
    let mut smells = match lang {
        Language::JavaScript | Language::TypeScript => js_ts_default(),
        Language::Python | Language::Rust => Smells::default(),
    };
    smells.disabled.extend(registry::default_off_smells(lang));
    let write_methods = registry::smell_wording(lang).write_methods;
    smells
        .name_promise_write_methods
        .extend(write_methods.iter().map(|method| method.to_string()));
    smells
}

/// JS/TS: enable `large_class` (no native method-count rule) and switch off the
/// always-on smells ESLint/SonarJS own (`max-depth`, cognitive complexity). The
/// remaining linter-owned smells are already off via shared metric toggles
/// (`cyclomatic_complexity`, `long_function`, `long_parameter_list`,
/// `magic_numbers`), so a user can still opt into them per language.
fn js_ts_default() -> Smells {
    let mut smells = Smells {
        large_class: true,
        ..Smells::default()
    };
    smells
        .disabled
        .extend([SmellKind::HighCognitiveComplexity, SmellKind::DeepNesting]);
    smells
}
