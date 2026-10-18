//! Language-neutral structural fingerprints for repeated guard detection.

use crate::profiles::typevocab::is_ident_char;
use crate::spine::ir::FunctionUnit;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use tree_sitter::Node;

/// Lines within which a repeated guard condition counts as a repeat.
const REPEAT_WINDOW_LINES: usize = 5;

struct Guard {
    line: usize,
    names: Vec<String>,
}

/// Guards seen on the current path, keyed by scope and condition.
pub(crate) struct GuardLog {
    guards: HashMap<u64, Guard>,
    identifier_chars: &'static [char],
}

impl GuardLog {
    /// An empty log for a language whose identifiers may also contain `identifier_chars`.
    pub(crate) fn new(identifier_chars: &'static [char]) -> Self {
        Self {
            guards: HashMap::new(),
            identifier_chars,
        }
    }

    /// Record an `if`'s condition keyed by its nearest `scope_kinds` ancestor; count a repeat within 5 lines.
    pub(crate) fn record(
        &mut self,
        unit: &mut FunctionUnit,
        branch: Node,
        scope_kinds: &[&str],
        src: &[u8],
    ) {
        let Some(condition) = branch.child_by_field_name("condition") else {
            return;
        };
        let mut ancestors = std::iter::successors(branch.parent(), Node::parent);
        let scope = ancestors
            .clone()
            .find(|ancestor| scope_kinds.contains(&ancestor.kind()));
        let ancestor = ancestors
            .find(|ancestor| ancestor.kind() == branch.kind())
            .and_then(|parent| parent.child_by_field_name("condition"));
        let line = condition.start_position().row + 1;
        let context = ancestor.map(|node| fingerprint(node, src)).unwrap_or(0);
        let key = combine(
            combine(scope.map_or(0, |node| node.id() as u64), context),
            fingerprint(condition, src),
        );
        let guard = Guard {
            line,
            names: names(condition, src, self.identifier_chars),
        };
        if self
            .guards
            .insert(key, guard)
            .is_some_and(|previous| line.saturating_sub(previous.line) <= REPEAT_WINDOW_LINES)
        {
            unit.review_risks.repeated_guards += 1;
        }
    }

    /// State may change across a suspension point.
    pub(crate) fn clear(&mut self) {
        self.guards.clear();
    }

    /// Drop guards whose condition mentions a name written by `target`.
    pub(crate) fn forget_written(&mut self, target: Node, src: &[u8]) {
        let written = names(target, src, self.identifier_chars);
        self.guards
            .retain(|_, guard| !guard.names.iter().any(|name| written.contains(name)));
    }
}

/// Identifier-like leaf tokens under `node`.
fn names(node: Node<'_>, src: &[u8], extra: &[char]) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![node];
    while let Some(current) = stack.pop() {
        if current.child_count() == 0 {
            let text = current.utf8_text(src).unwrap_or_default();
            if is_name(text, extra) {
                out.push(text.to_string());
            }
            continue;
        }
        let mut cursor = current.walk();
        stack.extend(current.children(&mut cursor));
    }
    out
}

fn is_name(text: &str, extra: &[char]) -> bool {
    text.chars().next().is_some_and(|first| !first.is_numeric())
        && text.chars().all(|c| is_ident_char(c, extra))
}

fn fingerprint(node: Node<'_>, src: &[u8]) -> u64 {
    fn hash_node(node: Node<'_>, src: &[u8], state: &mut DefaultHasher) {
        node.kind().hash(state);
        if node.child_count() == 0 {
            node.utf8_text(src).unwrap_or_default().hash(state);
            return;
        }
        for index in 0..node.child_count() {
            if let Some(child) = node.child(index) {
                hash_node(child, src, state);
            }
        }
    }

    let mut state = DefaultHasher::new();
    hash_node(node, src, &mut state);
    state.finish()
}

fn combine(ancestor: u64, condition: u64) -> u64 {
    let mut state = DefaultHasher::new();
    ancestor.hash(&mut state);
    condition.hash(&mut state);
    state.finish()
}
