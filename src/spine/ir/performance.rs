//! Language-neutral facts for performance-oriented smell detection.

use super::LoopFrame;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct PerformanceFacts {
    pub loops: Vec<PerfLine>,
    pub nested_loops: Vec<PerfLine>,
    pub sorts_in_loops: Vec<PerfLine>,
    pub iteration_calls: Vec<CallFact>,
    pub calls: Vec<CallFact>,
    pub loop_calls: Vec<CallFact>,
    /// End byte offsets of `return` / `throw` in this unit's own body.
    pub exits: Vec<usize>,
    /// Declared local names with the byte offset where the declaration ends.
    pub bindings: Vec<(String, usize)>,
    pub loop_frames: Vec<LoopFrame>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct PerfLine {
    pub line: usize,
    pub subject: String,
    /// Innermost enclosing loop frame; a nested loop's own frame.
    pub frame: Option<usize>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CallFact {
    pub target: String,
    pub base: String,
    pub method: String,
    pub line: usize,
    pub member: bool,
    /// Nearest conditional branch start line; zero means unconditional.
    pub region: usize,
    /// Start byte of the call expression.
    pub offset: usize,
    pub pass: IterationPass,
    /// Nested in an `expect(...)` / `assert(...)` call.
    pub in_assertion: bool,
    /// Identifiers read by the call's arguments.
    pub arg_idents: Vec<String>,
    /// Innermost enclosing loop frame.
    pub frame: Option<usize>,
    pub awaited: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IterationPass {
    #[default]
    Full,
    /// `map(x => x.field)`.
    Projection,
}

impl CallFact {
    pub fn named(name: &str, line: usize) -> Self {
        CallFact {
            target: name.to_string(),
            method: name.to_string(),
            line,
            ..CallFact::default()
        }
    }

    pub fn member(base: &str, method: &str, line: usize) -> Self {
        CallFact {
            target: format!("{base}.{method}"),
            base: base.to_string(),
            method: method.to_string(),
            line,
            member: true,
            ..CallFact::default()
        }
    }
}
