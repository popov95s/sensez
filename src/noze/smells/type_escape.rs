//! `type_escape_cast`: casts and suppressions that silence the type checker
//! inside a function body. Signatures belong to `loose_typing`.

use super::lines::{hit_lines, line_list};
use super::{make, SmellContext};
use crate::config::smells::{Smells, Strictness};
use crate::profiles::registry;
use crate::report::{Severity, SmellFinding, SmellKind};
use crate::spine::ir::{CommentClass, CommentSpan, FunctionUnit, TypeEscape, TypeEscapeKind};
use std::collections::HashSet;

pub fn detect(
    ctx: &SmellContext<'_>,
    metrics: &[FunctionUnit],
    comments: &[CommentSpan],
    cfg: &Smells,
    out: &mut Vec<SmellFinding>,
) {
    if !cfg.is_enabled(SmellKind::TypeEscapeCast)
        || ctx.test_source
        || registry::is_declaration_only(ctx.path)
    {
        return;
    }
    let explained: HashSet<usize> = comments
        .iter()
        .filter(|comment| comment.class == CommentClass::Exempt)
        .flat_map(|comment| [comment.end_line, comment.end_line + 1])
        .collect();
    out.extend(
        metrics
            .iter()
            .filter_map(|m| finding(ctx, m, &explained, cfg.type_escape_strictness)),
    );
}

/// One finding per function listing every reported escape by kind and line.
fn finding(
    ctx: &SmellContext<'_>,
    m: &FunctionUnit,
    explained: &HashSet<usize>,
    strictness: Strictness,
) -> Option<SmellFinding> {
    let guarded = ctx
        .type_hints
        .return_type(m)
        .and_then(|ty| ctx.type_vocabulary.guarded_param(ty));
    let escapes: Vec<&TypeEscape> = m
        .type_escapes
        .iter()
        .filter(|escape| reported(escape.kind, strictness))
        .filter(|escape| guarded.is_none() || escape.subject.as_deref() != guarded)
        .filter(|escape| is_suppression(escape.kind) || !explained.contains(&escape.line))
        .collect();
    let lines = hit_lines(escapes.iter().map(|escape| escape.line));
    let &first = lines.first()?;
    let span = m.end_line.saturating_sub(m.start_line) + 1;
    let count = escapes.len();
    let noun = if count == 1 {
        "type escape"
    } else {
        "type escapes"
    };
    let finding = make(
        SmellKind::TypeEscapeCast,
        format!(
            "{count} {noun} in {span} lines: {} — give the value a type or narrow it; a suppression that stays names its reason",
            by_label(&escapes)
        ),
        ctx.path,
        first,
        &m.name,
        Severity::Info,
        count as u32,
        0,
    );
    Some(SmellFinding { lines, ..finding })
}

fn reported(kind: TypeEscapeKind, strictness: Strictness) -> bool {
    match kind {
        TypeEscapeKind::AnyCast | TypeEscapeKind::BlanketSuppression => true,
        TypeEscapeKind::DoubleCast | TypeEscapeKind::UnexplainedSuppression => {
            strictness != Strictness::Low
        }
        TypeEscapeKind::NonNullCall => strictness == Strictness::High,
    }
}

fn is_suppression(kind: TypeEscapeKind) -> bool {
    matches!(
        kind,
        TypeEscapeKind::BlanketSuppression | TypeEscapeKind::UnexplainedSuppression
    )
}

/// "`as any` at L12, L15; `as unknown as` at L18", labels in first-seen order.
fn by_label(escapes: &[&TypeEscape]) -> String {
    let mut labels: Vec<&str> = Vec::new();
    for escape in escapes {
        if !labels.contains(&escape.label.as_str()) {
            labels.push(&escape.label);
        }
    }
    labels
        .iter()
        .map(|label| {
            let lines = hit_lines(
                escapes
                    .iter()
                    .filter(|escape| escape.label == *label)
                    .map(|escape| escape.line),
            );
            format!("`{label}` at {}", line_list(&lines))
        })
        .collect::<Vec<_>>()
        .join("; ")
}
