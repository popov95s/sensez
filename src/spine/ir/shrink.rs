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
        self.compact_rare();
        self.param_names.shrink_to_fit();
        self.chain_hits.shrink_to_fit();
        self.schema_calls.shrink_to_fit();
        self.review_risks.helper_calls.shrink_to_fit();
        self.performance.shrink_to_fit();
    }
}

impl FunctionUnit {
    fn compact_rare(&mut self) {
        self.destructured_params.compact();
        self.collapsible_nested_if_lines.compact();
        self.nested_ternaries.compact();
        self.local_reassigns.compact();
        self.self_attrs.compact();
        self.own_method_calls.compact();
        self.str_keys.compact();
        self.destructured_keys.compact();
        self.str_key_lines.compact();
        self.validated_names.compact();
        self.returned_constructors.compact();
        self.returned_names.compact();
        self.mutated_names.compact();
        self.attr_mutated_names.compact();
        self.method_mutations.compact();
        self.literal_memberships.compact();
        self.dynamic_key_receivers.compact();
        self.type_escapes.compact();
        self.state_writes.compact();
        self.string_fallbacks.compact();
        self.mutation_lines.compact();
        self.magic_numbers.compact();
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
