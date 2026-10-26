//! Per-loop context a walker records while inside a loop body.

use super::{PerfLine, PerformanceFacts};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoopKind {
    #[default]
    ForOf,
    ForIndex,
    While,
    /// Callback passed to a per-item iteration method (`xs.map(x => ...)`).
    IterCallback,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoopBound {
    #[default]
    Unbounded,
    /// Literal collection or constant-named subject.
    Fixed,
    /// Literal trip count (`i < 3`, `range(3)`, `0..3`).
    Count(usize),
}

impl LoopBound {
    pub fn within(self, max_count: usize) -> bool {
        match self {
            LoopBound::Unbounded => false,
            LoopBound::Fixed => true,
            LoopBound::Count(count) => count <= max_count,
        }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct LoopFrame {
    pub kind: LoopKind,
    pub parent: Option<usize>,
    pub bindings: Vec<String>,
    pub subject: String,
    pub bound: LoopBound,
    pub cond_idents: Vec<String>,
    /// Names reassigned in the body from a value fed by a call.
    pub advanced: BTreeSet<String>,
    pub declared: BTreeSet<String>,
    /// Body locals that hold a call's result.
    pub call_results: BTreeSet<String>,
}

impl LoopFrame {
    pub fn is_statement(&self) -> bool {
        self.kind != LoopKind::IterCallback
    }

    /// `name` is bound by the loop header or declared in its body.
    pub fn owns(&self, name: &str) -> bool {
        self.bindings.iter().any(|b| b == name) || self.declared.contains(name)
    }

    /// A `while` / `do` loop whose condition variable is refreshed from a call in the body.
    pub fn is_paging(&self) -> bool {
        self.kind == LoopKind::While && self.cond_idents.iter().any(|c| self.advanced.contains(c))
    }
}

impl PerformanceFacts {
    pub fn open_frame(&mut self, frame: LoopFrame) -> usize {
        self.loop_frames.push(frame);
        self.loop_frames.len() - 1
    }

    /// Record a loop statement and its frame; a loop inside another is also nested.
    pub fn open_loop(&mut self, line: PerfLine, frame: LoopFrame) -> usize {
        let nested = self.in_loop_statement(frame.parent);
        let id = self.open_frame(frame);
        if nested {
            self.nested_loops.push(PerfLine {
                frame: Some(id),
                ..line.clone()
            });
        }
        self.loops.push(line);
        id
    }

    /// `innermost` and every enclosing frame, innermost first.
    pub fn frames(&self, innermost: Option<usize>) -> impl Iterator<Item = &LoopFrame> {
        std::iter::successors(innermost.and_then(|id| self.loop_frames.get(id)), |frame| {
            frame.parent.and_then(|id| self.loop_frames.get(id))
        })
    }

    pub fn in_loop_statement(&self, innermost: Option<usize>) -> bool {
        innermost
            .and_then(|id| self.loop_frames.get(id))
            .is_some_and(LoopFrame::is_statement)
    }

    pub fn note_advanced(&mut self, innermost: Option<usize>, names: &[String]) {
        let mut next = innermost;
        while let Some(id) = next {
            let frame = &mut self.loop_frames[id];
            frame.advanced.extend(names.iter().cloned());
            next = frame.parent;
        }
    }

    pub fn note_call_results(&mut self, innermost: Option<usize>, names: &[String]) {
        self.note_in_frame(innermost, names, |frame| &mut frame.call_results);
    }

    pub fn is_call_result(&self, innermost: Option<usize>, name: &str) -> bool {
        self.frames(innermost)
            .any(|frame| frame.call_results.contains(name))
    }

    pub fn note_declared(&mut self, innermost: Option<usize>, names: &[String]) {
        self.note_in_frame(innermost, names, |frame| &mut frame.declared);
    }

    fn note_in_frame(
        &mut self,
        innermost: Option<usize>,
        names: &[String],
        set: fn(&mut LoopFrame) -> &mut BTreeSet<String>,
    ) {
        if let Some(id) = innermost {
            set(&mut self.loop_frames[id]).extend(names.iter().cloned());
        }
    }
}
