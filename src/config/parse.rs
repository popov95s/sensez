//! Strict parsing of a discovered config source into a [`Config`].

use super::model::Config;
use super::problem::ConfigProblem;
use super::source::Source;
use super::suggest::suggest;
use super::unknown_keys::{self, KeyPolicy, UnknownKey};
use serde::Deserialize;
use std::ops::Range;
use toml_edit::{ImDocument, Item, TableLike};

#[derive(Deserialize)]
struct Pyproject {
    tool: PyprojectTool,
}

#[derive(Deserialize)]
struct PyprojectTool {
    sensez: Config,
}

/// Parse `source`; `Ok` carries warnings, `Err` every error found.
pub(super) fn parse(source: &Source) -> Result<(Config, Vec<ConfigProblem>), Vec<ConfigProblem>> {
    let text = match source {
        Source::SensezToml(text) | Source::Pyproject(text) => text.as_str(),
        Source::Defaults => return Ok((Config::default(), Vec::new())),
    };
    let file = source.file();
    let at = |span: Option<Range<usize>>, message: String| ConfigProblem {
        file,
        position: span.map(|span| position(text, span.start)),
        message,
    };
    let doc = ImDocument::parse(text).map_err(|err| vec![at(err.span(), err.message().into())])?;
    let table = match source {
        Source::Pyproject(_) => match tool_sensez(doc.as_table()) {
            Some(table) => table,
            None => return Ok((Config::default(), Vec::new())),
        },
        _ => doc.as_table(),
    };
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    for key in unknown_keys::collect(table) {
        let problem = at(key.span.clone(), unknown_key_message(&key));
        match key.policy {
            KeyPolicy::Reject => errors.push(problem),
            KeyPolicy::Warn => warnings.push(problem),
        }
    }
    let parsed = match source {
        Source::Pyproject(_) => toml::from_str::<Pyproject>(text).map(|p| p.tool.sensez),
        _ => toml::from_str::<Config>(text),
    };
    if let Err(err) = &parsed {
        errors.push(at(err.span(), with_variant_suggestion(err.message())));
    }
    errors.sort_by_key(|problem| problem.position);
    warnings.sort_by_key(|problem| problem.position);
    match parsed {
        Ok(config) if errors.is_empty() => Ok((config, warnings)),
        _ => Err(errors),
    }
}

fn tool_sensez(doc: &toml_edit::Table) -> Option<&dyn TableLike> {
    doc.get("tool")
        .and_then(Item::as_table_like)?
        .get("sensez")
        .and_then(Item::as_table_like)
}

fn unknown_key_message(key: &UnknownKey) -> String {
    let hint = key
        .suggestion
        .as_deref()
        .map(|s| format!(" (did you mean `{s}`?)"))
        .unwrap_or_default();
    match key.policy {
        KeyPolicy::Reject => format!("unknown key `{}`{hint}", key.path),
        KeyPolicy::Warn => format!("unknown key `{}` ignored{hint}", key.path),
    }
}

/// Append a "did you mean" to serde's `unknown variant` messages.
fn with_variant_suggestion(message: &str) -> String {
    let suggestion = message
        .split_once("unknown variant `")
        .and_then(|(_, rest)| rest.split_once('`'))
        .and_then(|(given, expected)| suggest(given, expected.split('`').skip(1).step_by(2)));
    match suggestion {
        Some(s) => format!("{message} (did you mean `{s}`?)"),
        None => message.to_string(),
    }
}

/// 1-indexed (line, column) of byte `offset` in `text`.
fn position(text: &str, offset: usize) -> (usize, usize) {
    let before = &text[..offset.min(text.len())];
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    (
        before.matches('\n').count() + 1,
        before[line_start..].chars().count() + 1,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_is_one_indexed() {
        assert_eq!(position("a = 1\nbb = 2\n", 6), (2, 1));
        assert_eq!(position("a = 1\nbb = 2\n", 11), (2, 6));
    }

    #[test]
    fn unknown_variant_gets_a_suggestion() {
        let message =
            "unknown variant `must_fx`, expected one of `info`, `advisory`, `warning`, `must_fix`";
        assert!(with_variant_suggestion(message).ends_with("(did you mean `must_fix`?)"));
        assert_eq!(
            with_variant_suggestion("unknown variant `critical`, expected `a` or `b`"),
            "unknown variant `critical`, expected `a` or `b`"
        );
    }
}
