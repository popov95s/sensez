//! String defaults (`{ s = "" }`, `name = ""`) that the same function compares to that literal.

use super::units::is_function;
use crate::profiles::node_names::NodeNames;
use crate::profiles::walk;
use crate::spine::ir::{FallbackSink, StringFallback};
use tree_sitter::Node;

const EQUALITY: &[&str] = &["===", "!==", "==", "!="];

/// String defaults and literal comparisons of one function, gathered as its walk reaches them.
#[derive(Default)]
pub(super) struct ComparedDefaults {
    defaults: Vec<Default>,
    comparisons: Vec<Comparison>,
}

impl ComparedDefaults {
    /// Defaults outside `func`'s body; the walk feeds the body through [`Self::scan`].
    pub(super) fn start(func: Node, src: &[u8]) -> Self {
        let mut found = Self::default();
        let body = func.field("body").map(|body| body.id());
        let mut cursor = func.walk();
        for child in func.named_children(&mut cursor) {
            if Some(child.id()) != body {
                found.collect(child, src);
            }
        }
        found
    }

    pub(super) fn scan(&mut self, node: Node, src: &[u8]) {
        self.defaults.extend(string_default(node, src));
        self.comparisons.extend(literal_comparison(node, src));
    }

    /// The defaults the function also compares against.
    pub(super) fn finish(self) -> Vec<StringFallback> {
        let comparisons = self.comparisons;
        self.defaults
            .into_iter()
            .filter(|(name, literal, _)| comparisons.contains(&(name.clone(), literal.clone())))
            .map(|(_, literal, line)| StringFallback {
                line,
                literal,
                sink: FallbackSink::Compared,
                target: String::new(),
            })
            .collect()
    }

    fn collect(&mut self, node: Node, src: &[u8]) {
        if is_function(node.kind_name()) {
            return;
        }
        self.scan(node, src);
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.collect(child, src);
        }
    }
}

type Default = (String, String, usize);
type Comparison = (String, String);

fn string_default(node: Node, src: &[u8]) -> Option<Default> {
    let (name, value) = match node.kind_name() {
        "object_assignment_pattern" | "assignment_pattern" => {
            (node.field("left")?, node.field("right")?)
        }
        "required_parameter" | "optional_parameter" => {
            (node.field("pattern")?, node.field("value")?)
        }
        _ => return None,
    };
    let name = walk::node_text(name, src)?;
    let literal = string_body(value, src)?;
    Some((name.to_string(), literal, node.start_position().row + 1))
}

/// `name === "literal"` in either order.
fn literal_comparison(node: Node, src: &[u8]) -> Option<Comparison> {
    if node.kind_name() != "binary_expression" {
        return None;
    }
    let operator = walk::node_text(node.field("operator")?, src)?;
    if !EQUALITY.contains(&operator) {
        return None;
    }
    let left = node.field("left")?;
    let right = node.field("right")?;
    let (name, literal) = match (left.kind_name(), right.kind_name()) {
        ("identifier", "string") => (left, right),
        ("string", "identifier") => (right, left),
        _ => return None,
    };
    Some((
        walk::node_text(name, src)?.to_string(),
        string_body(literal, src)?,
    ))
}

fn string_body(node: Node, src: &[u8]) -> Option<String> {
    if node.kind_name() != "string" {
        return None;
    }
    let text = walk::node_text(node, src)?;
    Some(text.get(1..text.len().checked_sub(1)?)?.to_string())
}
