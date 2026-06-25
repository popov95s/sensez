//! Cohesion smells: Divergent Change (via LCOM).
//!
//! LCOM here is component-based (LCOM4-style): methods are nodes, linked when
//! they share a `self.<attr>`. A class whose methods fall into several disjoint
//! components is poorly cohesive — a hazard for divergent change.
//!
//! Only methods that actually touch instance state participate. Stateless
//! classes (CRUD repos of `@staticmethod`s, Pydantic/`Enum` data classes) carry
//! no cohesion signal, so they are skipped rather than flagged as a wall of
//! one-method "islands".

use super::make;
use super::union_find::{find, union};
use super::SmellContext;
use crate::report::{Severity, SmellFinding, SmellKind};
use crate::spine::ir::{ClassUnit, FunctionMetrics};
use std::collections::HashMap;

/// Minimum instance-stateful methods before LCOM is meaningful. With one or
/// two, the component count can only be 1 or 2, and two methods on disjoint
/// attributes is ordinary (a getter/setter pair) — only from three methods up
/// does "the methods split into islands" carry a divergent-change signal.
const MIN_STATEFUL_METHODS: usize = 3;

pub fn detect(
    ctx: &SmellContext<'_>,
    _metrics: &[FunctionMetrics],
    classes: &[ClassUnit],
    out: &mut Vec<SmellFinding>,
) {
    for class in classes {
        divergent_change(ctx, class, out);
    }
}

fn divergent_change(ctx: &SmellContext<'_>, class: &ClassUnit, out: &mut Vec<SmellFinding>) {
    // Data classes (Pydantic / settings / enums) hold fields, not behavior.
    if class.bases.iter().any(|b| is_data_base(b)) {
        return;
    }
    // Methods that genuinely use instance state (ignore static/util methods that
    // never touch `self`, which would otherwise read as cohesion "islands").
    let stateful: Vec<&str> = class
        .method_attr_use
        .iter()
        .filter(|(_, attrs)| !attrs.is_empty())
        .map(|(name, _)| name.as_str())
        .collect();
    if stateful.len() < MIN_STATEFUL_METHODS {
        return;
    }

    let components = lcom_components(class, &stateful);
    if components >= 2 {
        out.push(make(
            SmellKind::DivergentChange,
            format!(
                "class splits into {components} cohesion clusters (low LCOM) — \
                 prone to divergent change"
            ),
            ctx.path,
            class.start_line,
            &class.name,
            Severity::Warning,
            components as u32,
            1,
        ));
    }
}

/// Connected components among `stateful` methods linked by a shared `self.<attr>`.
fn lcom_components(class: &ClassUnit, stateful: &[&str]) -> usize {
    let mut parent: Vec<usize> = (0..stateful.len()).collect();
    let mut owner: HashMap<&str, usize> = HashMap::new();
    for (i, m) in stateful.iter().enumerate() {
        if let Some(attrs) = class.method_attr_use.get(*m) {
            for attr in attrs {
                match owner.get(attr.as_str()) {
                    Some(&j) => union(&mut parent, i, j),
                    None => {
                        owner.insert(attr.as_str(), i);
                    }
                }
            }
        }
    }
    (0..stateful.len())
        .filter(|&i| find(&mut parent, i) == i)
        .count()
}

/// Base classes that mark a class as a data container (fields, not behavior).
fn is_data_base(base: &str) -> bool {
    matches!(
        base.rsplit('.').next(),
        Some("BaseModel")
            | Some("BaseSettings")
            | Some("Enum")
            | Some("IntEnum")
            | Some("StrEnum")
            | Some("Protocol")
    )
}
