use super::union_find::{find, union};
use super::SmellContext;
use super::{make, Measure};
use crate::report::{Severity, SmellFinding, SmellKind};
use crate::spine::ir::{ClassUnit, FunctionUnit};
use std::collections::HashMap;

/// Minimum instance-stateful methods before LCOM is meaningful.
const MIN_STATEFUL_METHODS: usize = 3;

pub fn detect(
    ctx: &SmellContext<'_>,
    _metrics: &[FunctionUnit],
    classes: &[ClassUnit],
    out: &mut Vec<SmellFinding>,
) {
    for class in classes {
        divergent_change(ctx, class, out);
    }
}

fn divergent_change(ctx: &SmellContext<'_>, class: &ClassUnit, out: &mut Vec<SmellFinding>) {
    if class.bases.iter().any(|b| is_data_base(b)) || !class.implements.is_empty() {
        return;
    }
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
            Measure::new(components, 1),
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
        if let Some(calls) = class.method_calls.get(*m) {
            for called in calls {
                if let Some(j) = stateful.iter().position(|name| *name == called) {
                    union(&mut parent, i, j);
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
