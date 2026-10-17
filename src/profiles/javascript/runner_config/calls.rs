use super::evaluate::{named_children, Evaluator, Scope};
use super::value::{merge, Value};
use crate::profiles::javascript::roots::normalize as normalize_path;
use std::path::PathBuf;
use tree_sitter::Node;

/// Value of a config-helper call such as `defineConfig`, `mergeConfig` or `path.resolve`.
pub fn evaluate<'t>(evaluator: &Evaluator<'t>, call: Node<'t>, scope: &Scope<'_, 't>) -> Value {
    let Some(callee) = call.child_by_field_name("function") else {
        return evaluator.unreadable(call);
    };
    let arguments = call
        .child_by_field_name("arguments")
        .map(named_children)
        .unwrap_or_default();
    let argument = |index: usize| {
        arguments
            .get(index)
            .map_or(Value::Other, |node| evaluator.eval(*node, scope))
    };
    let name = evaluator.script.text(callee);
    match name.as_str() {
        "defineConfig" | "defineProject" | "defineWorkspace" => argument(0),
        "mergeConfig" => merge(argument(0), argument(1)),
        "require" => match argument(0) {
            Value::Str(specifier) if specifier.starts_with('.') => {
                evaluator.follow(&specifier, "default")
            }
            _ => evaluator.unreadable(call),
        },
        "path.resolve" | "resolve" | "path.posix.resolve" => {
            join_paths(evaluator.directory(), arguments.len(), argument)
        }
        "path.join" | "join" | "path.posix.join" => {
            join_paths(PathBuf::new(), arguments.len(), argument)
        }
        "fileURLToPath" => arguments
            .first()
            .and_then(|argument| file_url(evaluator, *argument, scope))
            .unwrap_or_else(|| evaluator.unreadable(call)),
        _ => Value::Opaque(format!("calls `{name}`")),
    }
}

fn join_paths(start: PathBuf, count: usize, argument: impl Fn(usize) -> Value) -> Value {
    let mut path = start;
    for index in 0..count {
        match argument(index) {
            Value::Str(part) => path.push(part),
            Value::Opaque(reason) => return Value::Opaque(reason),
            _ => return Value::Opaque("path helper has a non-string argument".into()),
        }
    }
    Value::Str(normalize_path(&path).to_string_lossy().into_owned())
}

/// `fileURLToPath(new URL("./x", import.meta.url))` or `fileURLToPath(import.meta.url)`.
fn file_url<'t>(
    evaluator: &Evaluator<'t>,
    argument: Node<'t>,
    scope: &Scope<'_, 't>,
) -> Option<Value> {
    if evaluator.script.text(argument) == "import.meta.url" {
        return Some(Value::Str(
            evaluator.script.path.to_string_lossy().into_owned(),
        ));
    }
    if argument.kind() != "new_expression" {
        return None;
    }
    let url_arguments = named_children(argument.child_by_field_name("arguments")?);
    let [specifier, base] = url_arguments.as_slice() else {
        return None;
    };
    if evaluator.script.text(*base) != "import.meta.url" {
        return None;
    }
    match evaluator.eval(*specifier, scope) {
        Value::Str(relative) => Some(Value::Str(
            normalize_path(&evaluator.directory().join(relative))
                .to_string_lossy()
                .into_owned(),
        )),
        _ => None,
    }
}
