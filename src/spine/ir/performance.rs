//! Language-neutral facts for performance-oriented smell detection.

#[derive(Debug, Clone, Default)]
pub struct PerformanceFacts {
    pub loops: Vec<PerfLine>,
    pub nested_loops: Vec<PerfLine>,
    pub sorts_in_loops: Vec<PerfLine>,
    pub iteration_calls: Vec<CallFact>,
    pub calls: Vec<CallFact>,
    pub loop_calls: Vec<CallFact>,
}

#[derive(Debug, Clone, Default)]
pub struct PerfLine {
    pub line: usize,
    pub subject: String,
}

#[derive(Debug, Clone, Default)]
pub struct CallFact {
    pub target: String,
    pub base: String,
    pub method: String,
    pub line: usize,
    pub member: bool,
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
        }
    }
}
