//! Type-assisted coupling smells: Feature Envy and Inappropriate Intimacy.
//!
//! Both need to resolve a receiver's type. When the type is unknown (no
//! annotation, no obvious instantiation) the smell is skipped — we never guess.

use super::make;
use crate::config::smells::Smells;
use crate::noze::{Severity, SmellFinding, SmellKind};
use crate::spine::parser::{ParsedFile, TypeHints};
use std::collections::HashSet;

/// Minimum external member touches before envy is worth reporting. One or two
/// touches of another object is ordinary collaboration; three+ dereferences of
/// the same foreign receiver inside one method is the classic envy signature
/// (Fowler's rule of three), and in practice the lowest floor that doesn't
/// flag every delegating wrapper.
const ENVY_FLOOR: usize = 3;

pub fn detect(file: &ParsedFile, _cfg: &Smells, out: &mut Vec<SmellFinding>) {
    feature_envy(file, out);
    inappropriate_intimacy(file, out);
}

fn feature_envy(file: &ParsedFile, out: &mut Vec<SmellFinding>) {
    for func in &file.walked.units.functions {
        // Feature envy is about a *method* neglecting its own object's data. Free
        // functions (endpoints, helpers) have no "own data", so they can't envy.
        if !func.is_method {
            continue;
        }
        let self_uses = func.receiver_access.get("self").copied().unwrap_or(0);
        if self_uses == 0 {
            continue; // a method that never touches self has no own data to neglect
        }
        // The most-touched non-self receiver. Tie-break on the receiver name
        // (smallest wins) so the pick is deterministic: `receiver_access` is a
        // HashMap, and a bare `max_by_key` would otherwise return whichever tied
        // receiver iteration yields last — which, combined with the type-resolve
        // gate below, made the finding flip on/off run to run.
        let envied = func
            .receiver_access
            .iter()
            .filter(|(r, _)| r.as_str() != "self")
            .max_by(|(ra, na), (rb, nb)| na.cmp(nb).then_with(|| rb.cmp(ra)));
        let Some((receiver, &count)) = envied else {
            continue;
        };
        // Require a clear margin over own-data access to avoid borderline noise.
        if count < ENVY_FLOOR || count <= self_uses * 2 {
            continue;
        }
        // Resolve the receiver's type; skip if unknown (precision over recall).
        let Some(ty) = resolve_type(&func.name, receiver, &file.walked.units.type_hints) else {
            continue;
        };
        out.push(make(
            SmellKind::FeatureEnvy,
            format!(
                "accesses `{receiver}` ({ty}) {count}× vs. own data {self_uses}× — \
                 behavior may belong on {ty}"
            ),
            &file.path,
            func.start_line,
            &func.name,
            Severity::Warning,
            count as u32,
            self_uses as u32,
        ));
    }
}

fn inappropriate_intimacy(file: &ParsedFile, out: &mut Vec<SmellFinding>) {
    let locals: HashSet<&str> = file
        .walked
        .units
        .classes
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    for (base, attrs) in &file.walked.usage.attribute_accesses {
        if base == "self" {
            continue;
        }
        let Some(ty) = file.walked.units.type_hints.var_types.get(base) else {
            continue;
        };
        if !locals.contains(ty.as_str()) {
            continue; // only flag intimacy with a class defined in this module
        }
        let mut privates: Vec<&str> = attrs
            .iter()
            .filter(|a| a.starts_with('_') && !a.starts_with("__"))
            .map(String::as_str)
            .collect();
        privates.sort(); // attrs is a HashSet — sort for a stable message string
        if !privates.is_empty() {
            out.push(make(
                SmellKind::InappropriateIntimacy,
                format!(
                    "`{base}` ({ty}) reaches into private member(s): {}",
                    join(&privates)
                ),
                &file.path,
                0,
                base,
                Severity::Info,
                privates.len() as u32,
                0,
            ));
        }
    }
}

/// Resolve `name` within `func`: a typed parameter first, else a typed/instantiated variable.
fn resolve_type(func: &str, name: &str, hints: &TypeHints) -> Option<String> {
    hints
        .param_types
        .get(&(func.to_string(), name.to_string()))
        .or_else(|| hints.var_types.get(name))
        .cloned()
}

fn join(items: &[&str]) -> String {
    items.join(", ")
}
