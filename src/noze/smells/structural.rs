//! Structural smells: deep nesting, long parameter lists, too many returns,
//! magic numbers, message chains (Law of Demeter), and split variables.

use super::exclusive_paths::max_on_one_path;
use super::lines::{hit_lines, line_list};
use super::{make, SmellContext};
use crate::config::smells::{Smells, Strictness};
use crate::report::{Severity, SmellFinding, SmellKind};
use crate::spine::ir::{FunctionUnit, MagicContext, MagicNumber};
use std::collections::BTreeMap;

pub fn detect(
    ctx: &SmellContext<'_>,
    metrics: &[FunctionUnit],
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
    fn inspect(&mut self, m: &FunctionUnit) {
        self.deep_nesting(m);
        self.long_params(m);
        self.too_many_returns(m);
        if self.cfg.magic_numbers {
            self.magic_numbers(m);
        }
        self.message_chains(m);
        self.unnecessary_nested_if(m);
        self.nested_ternary(m);
        if self.cfg.split_variable {
            self.split_variables(m);
        }
    }

    fn deep_nesting(&mut self, m: &FunctionUnit) {
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

    fn long_params(&mut self, m: &FunctionUnit) {
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

    fn too_many_returns(&mut self, m: &FunctionUnit) {
        let excluded = if self.cfg.count_guard_clauses {
            0
        } else {
            m.guard_returns
        };
        let counted = m.return_count - excluded;
        if !self.cfg.too_many_returns || counted <= self.cfg.max_returns {
            return;
        }
        let guards = if excluded > 0 {
            format!(", {excluded} guard clause(s) excluded")
        } else {
            String::new()
        };
        self.out.push(make(
            SmellKind::TooManyReturns,
            format!(
                "{counted} return statements (threshold {}{guards})",
                self.cfg.max_returns
            ),
            self.ctx.path,
            m.start_line,
            &m.name,
            Severity::Warning,
            counted as u32,
            self.cfg.max_returns as u32,
        ));
    }

    fn magic_numbers(&mut self, m: &FunctionUnit) {
        let sites: Vec<&MagicNumber> = m
            .magic_numbers
            .iter()
            .filter(|site| self.reports_magic(site))
            .collect();
        let Some(first) = sites.first() else {
            return;
        };
        let shown: Vec<String> = sites
            .iter()
            .take(3)
            .map(|site| format!("{}@L{}", site.text, site.line))
            .collect();
        let more = match sites.len().saturating_sub(3) {
            0 => String::new(),
            rest => format!(" (+{rest} more)"),
        };
        let finding = make(
            SmellKind::MagicNumbers,
            format!(
                "{} magic numeric literal(s): {}{more}",
                sites.len(),
                shown.join(", ")
            ),
            self.ctx.path,
            first.line,
            &m.name,
            Severity::Info,
            sites.len() as u32,
            0,
        );
        self.out.push(SmellFinding {
            lines: hit_lines(sites.iter().map(|site| site.line)),
            ..finding
        });
    }

    fn reports_magic(&self, site: &MagicNumber) -> bool {
        let waived = match site.context {
            MagicContext::Plain => false,
            MagicContext::ArrayIndex => self.cfg.magic_number_ignore_array_indexes,
            MagicContext::DefaultValue => self.cfg.magic_number_ignore_default_values,
            MagicContext::HttpStatus => self.cfg.magic_number_allow_http_status,
        };
        let value = site.text.parse::<f64>().ok();
        let ignored = self
            .cfg
            .magic_number_ignore
            .iter()
            .any(|ignore| ignore.parse::<f64>().ok() == value && value.is_some());
        !waived && !ignored
    }

    fn message_chains(&mut self, m: &FunctionUnit) {
        let hits: Vec<_> = m
            .chain_hits
            .iter()
            .filter(|hit| hit.depth > self.cfg.max_chain_depth)
            .filter(|hit| {
                hit.root
                    .as_ref()
                    .is_none_or(|root| !self.cfg.message_chain_ignore_roots.contains(root))
            })
            .collect();
        let Some(depth) = hits.iter().map(|hit| hit.depth).max() else {
            return;
        };
        let lines = hit_lines(hits.iter().map(|hit| hit.line));
        let Some(&first) = lines.first() else {
            return;
        };
        let finding = make(
            SmellKind::MessageChain,
            format!(
                "attribute chain depth {depth} (threshold {}) at {} — Law of Demeter",
                self.cfg.max_chain_depth,
                line_list(&lines)
            ),
            self.ctx.path,
            first,
            &m.name,
            Severity::Warning,
            depth as u32,
            self.cfg.max_chain_depth as u32,
        );
        self.out.push(SmellFinding { lines, ..finding });
    }

    fn unnecessary_nested_if(&mut self, m: &FunctionUnit) {
        let lines = hit_lines(m.collapsible_nested_if_lines.iter().copied());
        let Some(&line) = lines.first() else {
            return;
        };
        let count = m.collapsible_nested_if_lines.len();
        let finding = make(
            SmellKind::UnnecessaryNestedIf,
            format!(
                "{count} nested if(s) at {} can be combined with a boolean AND",
                line_list(&lines)
            ),
            self.ctx.path,
            line,
            &m.name,
            Severity::Info,
            count as u32,
            0,
        );
        self.out.push(SmellFinding { lines, ..finding });
    }

    fn nested_ternary(&mut self, m: &FunctionUnit) {
        let lexical = self.cfg.nested_ternary_strictness == Strictness::High;
        let counted: Vec<_> = m
            .nested_ternaries
            .iter()
            .filter(|ternary| ternary.depth > 0 || lexical)
            .collect();
        let mut chains: BTreeMap<usize, usize> = BTreeMap::new();
        for ternary in &counted {
            let levels = chains.entry(ternary.root_line).or_default();
            *levels = (*levels).max(ternary.depth.max(1) + 1);
        }
        let anchor = counted.iter().map(|ternary| ternary.line).min();
        let (Some(line), Some(&deepest)) = (anchor, chains.values().max()) else {
            return;
        };
        self.out.push(make(
            SmellKind::NestedTernary,
            format!(
                "{} nested ternary chain(s), deepest {deepest} levels — extract the result into a named helper or flatten it with if statements and early returns",
                chains.len()
            ),
            self.ctx.path,
            line,
            &m.name,
            Severity::Warning,
            deepest as u32,
            0,
        ));
    }

    /// Advisory: a local assigned `split_variable_min_assigns`+ times either holds
    /// distinct concepts or is branch-bound state — both want a single binding
    /// (extract a helper that returns the value).
    fn split_variables(&mut self, m: &FunctionUnit) {
        let min_assigns = self.cfg.split_variable_min_assigns.max(2);
        let mut locals: Vec<_> = m.local_reassigns.iter().collect();
        locals.sort_unstable_by_key(|(name, _)| *name);
        for (name, paths) in locals {
            let count = max_on_one_path(paths);
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
fn effective_params(m: &FunctionUnit) -> usize {
    let skip = matches!(
        m.param_names.first().map(String::as_str),
        Some("self") | Some("cls")
    ) as usize;
    m.param_names.len().saturating_sub(skip)
}
