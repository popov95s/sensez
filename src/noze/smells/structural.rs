//! Structural smells: deep nesting, long parameter lists, too many returns,
//! magic numbers, message chains (Law of Demeter), and split variables.

use super::{make, SmellContext};
use crate::config::smells::Smells;
use crate::report::{Severity, SmellFinding, SmellKind};
use crate::spine::ir::FunctionMetrics;

pub fn detect(
    ctx: &SmellContext<'_>,
    metrics: &[FunctionMetrics],
    cfg: &Smells,
    out: &mut Vec<SmellFinding>,
) {
    let mut detector = StructuralDetector { ctx, cfg, out };
    for m in metrics {
        detector.inspect(m);
    }
}

struct StructuralDetector<'a, 'b> {
    ctx: &'a SmellContext<'a>,
    cfg: &'a Smells,
    out: &'b mut Vec<SmellFinding>,
}

impl StructuralDetector<'_, '_> {
    fn inspect(&mut self, m: &FunctionMetrics) {
        self.deep_nesting(m);
        self.long_params(m);
        self.too_many_returns(m);
        if self.cfg.magic_numbers && m.magic_numbers > 0 {
            self.magic_numbers(m);
        }
        self.message_chains(m);
        self.unnecessary_nested_if(m);
        if self.cfg.split_variable {
            self.split_variables(m);
        }
    }

    fn deep_nesting(&mut self, m: &FunctionMetrics) {
        if m.max_nesting <= self.cfg.max_nesting {
            return;
        }
        self.out.push(make(
            SmellKind::DeepNesting,
            format!(
                "nesting depth {} (threshold {})",
                m.max_nesting, self.cfg.max_nesting
            ),
            self.ctx.path,
            m.start_line,
            &m.name,
            Severity::Warning,
            m.max_nesting as u32,
            self.cfg.max_nesting as u32,
        ));
    }

    fn long_params(&mut self, m: &FunctionMetrics) {
        let params = effective_params(m);
        if !self.cfg.long_parameter_list || params <= self.cfg.max_params {
            return;
        }
        self.out.push(make(
            SmellKind::LongParameterList,
            format!("{params} parameters (threshold {})", self.cfg.max_params),
            self.ctx.path,
            m.start_line,
            &m.name,
            Severity::Warning,
            params as u32,
            self.cfg.max_params as u32,
        ));
    }

    fn too_many_returns(&mut self, m: &FunctionMetrics) {
        if !self.cfg.too_many_returns || m.return_count <= self.cfg.max_returns {
            return;
        }
        self.out.push(make(
            SmellKind::TooManyReturns,
            format!(
                "{} return statements (threshold {})",
                m.return_count, self.cfg.max_returns
            ),
            self.ctx.path,
            m.start_line,
            &m.name,
            Severity::Warning,
            m.return_count as u32,
            self.cfg.max_returns as u32,
        ));
    }

    fn magic_numbers(&mut self, m: &FunctionMetrics) {
        self.out.push(make(
            SmellKind::MagicNumbers,
            format!("{} magic numeric literal(s)", m.magic_numbers),
            self.ctx.path,
            m.start_line,
            &m.name,
            Severity::Info,
            m.magic_numbers as u32,
            0,
        ));
    }

    fn message_chains(&mut self, m: &FunctionMetrics) {
        if m.max_chain_depth <= self.cfg.max_chain_depth {
            return;
        }
        self.out.push(make(
            SmellKind::MessageChain,
            format!(
                "attribute chain depth {} (threshold {}) — Law of Demeter",
                m.max_chain_depth, self.cfg.max_chain_depth
            ),
            self.ctx.path,
            m.start_line,
            &m.name,
            Severity::Warning,
            m.max_chain_depth as u32,
            self.cfg.max_chain_depth as u32,
        ));
    }

    fn unnecessary_nested_if(&mut self, m: &FunctionMetrics) {
        if m.collapsible_nested_ifs == 0 {
            return;
        }
        self.out.push(make(
            SmellKind::UnnecessaryNestedIf,
            format!(
                "{} nested if(s) can be combined with a boolean AND",
                m.collapsible_nested_ifs
            ),
            self.ctx.path,
            m.start_line,
            &m.name,
            Severity::Info,
            m.collapsible_nested_ifs as u32,
            0,
        ));
    }

    /// Advisory: a local assigned `split_variable_min_assigns`+ times either holds
    /// distinct concepts or is branch-bound state — both want a single binding
    /// (extract a helper that returns the value).
    fn split_variables(&mut self, m: &FunctionMetrics) {
        let min_assigns = self.cfg.split_variable_min_assigns.max(2);
        for (name, &count) in &m.local_reassigns {
            if count < min_assigns {
                continue;
            }
            self.out.push(make(
                SmellKind::SplitVariable,
                format!(
                    "local `{name}` assigned {count} times — bind it once (extract a helper returning the value)"
                ),
                self.ctx.path,
                m.start_line,
                &m.name,
                Severity::Info,
                count as u32,
                (min_assigns - 1) as u32,
            ));
        }
    }
}

/// Parameter count excluding a leading `self`/`cls` receiver.
fn effective_params(m: &FunctionMetrics) -> usize {
    let skip = matches!(
        m.param_names.first().map(String::as_str),
        Some("self") | Some("cls")
    ) as usize;
    m.param_names.len().saturating_sub(skip)
}
