//! Built-in per-language smell defaults.

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
    smells.name_promise_write_methods.extend(
        registry::default_write_methods(lang)
            .iter()
            .map(|method| method.to_string()),
    );
    smells
}

/// JS/TS.
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
