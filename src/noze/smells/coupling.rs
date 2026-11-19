use super::SmellContext;
use super::{make, Measure};
use crate::config::smells::Smells;
use crate::report::{Severity, SmellFinding, SmellKind};
use crate::spine::ir::{FunctionUnit, TypeHints};
use std::collections::{HashMap, HashSet};

/// Per-file view of cross-function facts only Feature Envy / Inappropriate Intimacy read.
pub(super) struct UsageFacts<'a> {
    /// Base identifier → distinct attribute names accessed on it (`obj.attr` per function, unioned file-wide).
    pub attribute_accesses: &'a HashMap<String, HashSet<String>>,
}

/// Minimum external member touches before envy is worth reporting.
const ENVY_FLOOR: usize = 3;

pub fn detect(
    ctx: &SmellContext<'_>,
    metrics: &[FunctionUnit],
    usage: &UsageFacts<'_>,
    locals: &HashSet<&str>,
    _cfg: &Smells,
    out: &mut Vec<SmellFinding>,
) {
    feature_envy(ctx, metrics, out);
    inappropriate_intimacy(ctx, usage, locals, out);
}

fn feature_envy(ctx: &SmellContext<'_>, metrics: &[FunctionUnit], out: &mut Vec<SmellFinding>) {
    for m in metrics {
        if !m.is_method || m.is_constructor {
            continue;
        }
        let self_uses = m.receiver_access.get("self").copied().unwrap_or(0);
        if self_uses == 0 || self_uses == m.self_writes {
            continue;
        }
        let envied = m
            .receiver_access
            .iter()
            .filter(|(r, _)| r.as_str() != "self")
            .max_by(|(ra, na), (rb, nb)| na.cmp(nb).then_with(|| rb.cmp(ra)));
        let Some((receiver, &count)) = envied else {
            continue;
        };
        if count < ENVY_FLOOR || count <= self_uses * 2 {
            continue;
        }
        let Some(ty) = resolve_type(m, receiver, ctx.type_hints)
            .filter(|ty| ctx.type_vocabulary.is_nominal_type(ty))
            .map(|ty| display_type(&ty))
        else {
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
            Measure::new(count, self_uses),
        ));
    }
}

fn inappropriate_intimacy(
    ctx: &SmellContext<'_>,
    usage: &UsageFacts<'_>,
    locals: &HashSet<&str>,
    out: &mut Vec<SmellFinding>,
) {
    for (base, attrs) in usage.attribute_accesses {
        if base == "self" {
            continue;
        }
        let Some(ty) = ctx.type_hints.var_types.get(base) else {
            continue;
        };
        if !locals.contains(ty.as_str()) {
            continue;
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
                Measure::new(privates.len(), 0),
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

/// Whitespace-collapsed type text, capped for one-line messages.
fn display_type(ty: &str) -> String {
    const MAX_CHARS: usize = 60;
    let collapsed = ty.split_whitespace().collect::<Vec<_>>().join(" ");
    match collapsed.char_indices().nth(MAX_CHARS) {
        Some((cut, _)) => format!("{}…", &collapsed[..cut]),
        None => collapsed,
    }
}

/// Resolve `name` within `func`: a typed parameter first, else a typed/instantiated variable.
fn resolve_type(func: &FunctionUnit, name: &str, hints: &TypeHints) -> Option<String> {
    hints
        .param_type(func, name)
        .or_else(|| hints.var_types.get(name))
        .cloned()
}
