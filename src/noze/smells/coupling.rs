//! Type-assisted coupling smells: Feature Envy and Inappropriate Intimacy.
//!
//! Both need to resolve a receiver's type. When the type is unknown (no
//! annotation, no obvious instantiation) the smell is skipped — we never guess.

use super::make;
use super::SmellContext;
use crate::config::smells::Smells;
use crate::report::{Severity, SmellFinding, SmellKind};
use crate::spine::ir::{FunctionMetrics, TypeHints};
use std::collections::{HashMap, HashSet};

/// Per-file view of cross-function facts only Feature Envy /
/// Inappropriate Intimacy read. Bundled so neither smell has to reach back
/// into the full `ParsedFile` just to fetch a usage table.
pub(super) struct UsageFacts {
    /// Base identifier → distinct attribute names accessed on it
    /// (`obj.attr` per function, unioned file-wide). Consumed by
    /// Inappropriate Intimacy to flag a non-`self` receiver reaching into
    /// the private members of a class defined in this file.
    pub attribute_accesses: HashMap<String, HashSet<String>>,
}

/// Minimum external member touches before envy is worth reporting. One or two
/// touches of another object is ordinary collaboration; three+ dereferences of
/// the same foreign receiver inside one method is the classic envy signature
/// (Fowler's rule of three), and in practice the lowest floor that doesn't
/// flag every delegating wrapper.
const ENVY_FLOOR: usize = 3;

pub fn detect(
    ctx: &SmellContext<'_>,
    metrics: &[FunctionMetrics],
    usage: &UsageFacts,
    locals: &HashSet<&str>,
    _cfg: &Smells,
    out: &mut Vec<SmellFinding>,
) {
    feature_envy(ctx, metrics, out);
    inappropriate_intimacy(ctx, usage, locals, out);
}

fn feature_envy(ctx: &SmellContext<'_>, metrics: &[FunctionMetrics], out: &mut Vec<SmellFinding>) {
    for m in metrics {
        // Feature envy is about a *method* neglecting its own object's data. Free
        // functions (endpoints, helpers) have no "own data", so they can't envy.
        if !m.is_method {
            continue;
        }
        let self_uses = m.receiver_access.get("self").copied().unwrap_or(0);
        if self_uses == 0 {
            continue; // a method that never touches self has no own data to neglect
        }
        // The most-touched non-self receiver. Tie-break on the receiver name
        // (smallest wins) so the pick is deterministic: `receiver_access` is a
        // HashMap, and a bare `max_by_key` would otherwise return whichever tied
        // receiver iteration yields last — which, combined with the type-resolve
        // gate below, made the finding flip on/off run to run.
        let envied = m
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
        let Some(ty) = resolve_type(&m.name, receiver, ctx.type_hints) else {
            continue;
        };
        out.push(make(
            SmellKind::FeatureEnvy,
            format!(
                "accesses `{receiver}` ({ty}) {count}× vs. own data {self_uses}× — \
                 behavior may belong on {ty}"
            ),
            ctx.path,
            m.start_line,
            &m.name,
            Severity::Warning,
            count as u32,
            self_uses as u32,
        ));
    }
}

fn inappropriate_intimacy(
    ctx: &SmellContext<'_>,
    usage: &UsageFacts,
    locals: &HashSet<&str>,
    out: &mut Vec<SmellFinding>,
) {
    for (base, attrs) in &usage.attribute_accesses {
        if base == "self" {
            continue;
        }
        let Some(ty) = ctx.type_hints.var_types.get(base) else {
            continue;
        };
        if !locals.contains(ty.as_str()) {
            continue; // only flag intimacy with a class defined in this module
        }
        let privates = private_attrs(attrs);
        if !privates.is_empty() {
            out.push(make(
                SmellKind::InappropriateIntimacy,
                format!(
                    "`{base}` ({ty}) reaches into private member(s): {}",
                    privates.join(", ")
                ),
                ctx.path,
                0,
                base,
                Severity::Info,
                privates.len() as u32,
                0,
            ));
        }
    }
}

fn private_attrs(attrs: &std::collections::HashSet<String>) -> Vec<&str> {
    let mut privates: Vec<&str> = attrs
        .iter()
        .filter(|a| a.starts_with('_') && !a.starts_with("__"))
        .map(String::as_str)
        .collect();
    privates.sort_unstable();
    privates
}

/// Resolve `name` within `func`: a typed parameter first, else a typed/instantiated variable.
fn resolve_type(func: &str, name: &str, hints: &TypeHints) -> Option<String> {
    hints
        .param_types
        .get(&(func.to_string(), name.to_string()))
        .or_else(|| hints.var_types.get(name))
        .cloned()
}
