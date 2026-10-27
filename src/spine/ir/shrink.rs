//! Dropping spare vector capacity from facts held for the whole scan.

use super::{FunctionUnit, PerformanceFacts, Walked};

impl Walked {
    pub fn shrink_to_fit(&mut self) {
        self.syntax.tokens.shrink_to_fit();
        self.syntax.spans.shrink_to_fit();
        self.syntax.lexemes.shrink_to_fit();
        self.units.functions.shrink_to_fit();
        self.units
            .functions
            .iter_mut()
            .for_each(FunctionUnit::shrink_to_fit);
    }
}

impl FunctionUnit {
    fn shrink_to_fit(&mut self) {
        self.param_names.shrink_to_fit();
        self.magic_numbers.shrink_to_fit();
        self.chain_hits.shrink_to_fit();
        self.schema_calls.shrink_to_fit();
        self.mutation_lines.shrink_to_fit();
        self.string_fallbacks.shrink_to_fit();
        self.state_writes.shrink_to_fit();
        self.review_risks.helper_calls.shrink_to_fit();
        self.performance.shrink_to_fit();
    }
}

impl PerformanceFacts {
    fn shrink_to_fit(&mut self) {
        self.loops.shrink_to_fit();
        self.iteration_calls.shrink_to_fit();
        self.calls.shrink_to_fit();
        self.loop_calls.shrink_to_fit();
        self.exits.shrink_to_fit();
        self.bindings.shrink_to_fit();
        self.loop_frames.shrink_to_fit();
    }
}
