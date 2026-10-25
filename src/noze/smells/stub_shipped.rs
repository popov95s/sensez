//! Shipped Stub: a production function whose body only defers its work.

use super::{make, SmellContext};
use crate::profiles::registry;
use crate::report::{Severity, SmellFinding, SmellKind};
use crate::spine::ir::{FunctionUnit, StubBody};

/// Name prefixes of conventional placeholder helpers (`todo()`, `unimplemented()`).
const PLACEHOLDER_NAMES: [&str; 3] = ["todo", "unimplemented", "not_implemented"];

pub(super) fn detect(
    ctx: &SmellContext<'_>,
    metrics: &[FunctionUnit],
    markers: &[String],
    out: &mut Vec<SmellFinding>,
) {
    if ctx.test_source || registry::is_declaration_only(ctx.path) {
        return;
    }
    for unit in metrics {
        let Some(stub) = &unit.review_risks.stub else {
            continue;
        };
        if is_placeholder_helper(&unit.name) || !is_marked(stub, markers) {
            continue;
        }
        out.push(make(
            SmellKind::StubShipped,
            format!(
                "`{}` ships a stub body ({}) — implement it or remove it",
                unit.name,
                describe(stub)
            ),
            ctx.path,
            unit.start_line,
            &unit.name,
            Severity::Warning,
            1,
            0,
        ));
    }
}

fn is_placeholder_helper(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    PLACEHOLDER_NAMES
        .iter()
        .any(|prefix| lower.starts_with(prefix))
}

fn is_marked(stub: &StubBody, markers: &[String]) -> bool {
    match stub {
        StubBody::Throws(message) => markers.iter().any(|marker| contains_words(message, marker)),
        StubBody::Placeholder | StubBody::NotImplemented | StubBody::TodoEmpty => true,
    }
}

/// `marker` appears in `message` on word boundaries, ignoring case.
fn contains_words(message: &str, marker: &str) -> bool {
    let text = message.to_lowercase();
    let word = marker.to_lowercase();
    text.match_indices(&word).any(|(start, _)| {
        let before = text[..start].chars().next_back();
        let after = text[start + word.len()..].chars().next();
        !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
    })
}

fn describe(stub: &StubBody) -> String {
    match stub {
        StubBody::Placeholder => "only a placeholder statement".to_string(),
        StubBody::NotImplemented => "only raises a not-implemented error".to_string(),
        StubBody::Throws(message) => format!("only throws \"{message}\""),
        StubBody::TodoEmpty => "empty under a TODO".to_string(),
    }
}
