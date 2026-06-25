//! Built-in per-language smell defaults.
//!
//! Python (and Rust, which has no unit extraction yet) use the baseline
//! [`Smells::default`]. JavaScript/TypeScript defer the rules ESLint /
//! typescript-eslint / SonarJS own and keep the heuristic, cross-file, and
//! type-discipline smells.

use super::knobs::Smells;
use crate::report::SmellKind;
use crate::spine::ir::Language;

/// The built-in default knob set for `lang`, before any `sensez.toml` overlay.
pub fn default_for(lang: Language) -> Smells {
    match lang {
        Language::JavaScript | Language::TypeScript => js_ts_default(),
        // Python: the canonical baseline. Rust: no unit extraction yet, so the
        // per-function smells produce nothing regardless; the baseline keeps the
        // graph smells (god_module / shotgun) live, matching today's behavior.
        Language::Python | Language::Rust => Smells::default(),
    }
}

/// JS/TS: enable `large_class` (no native method-count rule) and switch off the
/// always-on smells ESLint/SonarJS own (`max-depth`, cognitive complexity). The
/// remaining linter-owned smells are already off via shared metric toggles
/// (`cyclomatic_complexity`, `long_function`, `long_parameter_list`,
/// `magic_numbers`), so a user can still opt into them per language.
fn js_ts_default() -> Smells {
    Smells {
        large_class: true,
        disabled: vec![SmellKind::HighCognitiveComplexity, SmellKind::DeepNesting],
        ..Smells::default()
    }
}
