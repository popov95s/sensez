//! `loose_typing` on class fields: `private store: any`, `meta: dict[str, Any]`.

use super::typing::reportable_loose;
use super::{make, SmellContext};
use crate::config::smells::Smells;
use crate::profiles::typevocab::LooseTypeKind;
use crate::report::{Severity, SmellFinding, SmellKind};
use crate::spine::ir::{ClassProperty, ClassUnit};

pub(super) fn detect(
    ctx: &SmellContext<'_>,
    classes: &[ClassUnit],
    cfg: &Smells,
    out: &mut Vec<SmellFinding>,
) {
    for class in classes {
        let loose: Vec<(&ClassProperty, LooseTypeKind)> = class
            .properties
            .iter()
            .filter_map(|field| {
                reportable_loose(ctx, &field.type_name, cfg.loose_typing_strictness)
                    .map(|kind| (field, kind))
            })
            .collect();
        let Some((first, _)) = loose.first() else {
            continue;
        };
        let labels: Vec<String> = loose
            .iter()
            .map(|(field, _)| format!("{}: {}", field.name, field.type_name))
            .collect();
        let severity = if loose
            .iter()
            .any(|(_, kind)| *kind == LooseTypeKind::EscapeHatch)
        {
            Severity::Warning
        } else {
            Severity::Info
        };
        out.push(make(
            SmellKind::LooseTyping,
            format!(
                "fields [{}] — replace loose types with {}",
                labels.join(", "),
                ctx.wording.structure
            ),
            ctx.path,
            first.line,
            &class.name,
            severity,
            loose.len() as u32,
            0,
        ));
    }
}
