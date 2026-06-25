//! Structural smells: deep nesting, long parameter lists, too many returns,
//! magic numbers, message chains (Law of Demeter), and split variables.

use super::{make, SmellContext};
use crate::config::smells::Smells;
use crate::noze::{Severity, SmellFinding, SmellKind};
use crate::spine::ir::FunctionMetrics;

pub fn detect(
    ctx: &SmellContext<'_>,
    metrics: &[FunctionMetrics],
    cfg: &Smells,
    out: &mut Vec<SmellFinding>,
) {
    for m in metrics {
        deep_nesting(ctx, m, cfg, out);
        long_params(ctx, m, cfg, out);
        too_many_returns(ctx, m, cfg, out);
        if cfg.magic_numbers && m.magic_numbers > 0 {
            magic_numbers(ctx, m, out);
        }
        message_chains(ctx, m, cfg, out);
        unnecessary_nested_if(ctx, m, out);
        if cfg.split_variable {
            split_variables(ctx, m, cfg, out);
        }
    }
}

fn deep_nesting(
    ctx: &SmellContext<'_>,
    m: &FunctionMetrics,
    cfg: &Smells,
    out: &mut Vec<SmellFinding>,
) {
    if m.max_nesting > cfg.max_nesting {
        out.push(make(
            SmellKind::DeepNesting,
            format!("nesting depth {} (threshold {})", m.max_nesting, cfg.max_nesting),
            ctx.path,
            m.start_line,
            &m.name,
            Severity::Warning,
            m.max_nesting as u32,
            cfg.max_nesting as u32,
        ));
    }
}

fn long_params(
    ctx: &SmellContext<'_>,
    m: &FunctionMetrics,
    cfg: &Smells,
    out: &mut Vec<SmellFinding>,
) {
    let params = effective_params(m);
    if cfg.long_parameter_list && params > cfg.max_params {
        out.push(make(
            SmellKind::LongParameterList,
            format!("{params} parameters (threshold {})", cfg.max_params),
            ctx.path,
            m.start_line,
            &m.name,
            Severity::Warning,
            params as u32,
            cfg.max_params as u32,
        ));
    }
}

fn too_many_returns(
    ctx: &SmellContext<'_>,
    m: &FunctionMetrics,
    cfg: &Smells,
    out: &mut Vec<SmellFinding>,
) {
    if cfg.too_many_returns && m.return_count > cfg.max_returns {
        out.push(make(
            SmellKind::TooManyReturns,
            format!(
                "{} return statements (threshold {})",
                m.return_count, cfg.max_returns
            ),
            ctx.path,
            m.start_line,
            &m.name,
            Severity::Warning,
            m.return_count as u32,
            cfg.max_returns as u32,
        ));
    }
}

fn magic_numbers(ctx: &SmellContext<'_>, m: &FunctionMetrics, out: &mut Vec<SmellFinding>) {
    out.push(make(
        SmellKind::MagicNumbers,
        format!("{} magic numeric literal(s)", m.magic_numbers),
        ctx.path,
        m.start_line,
        &m.name,
        Severity::Info,
        m.magic_numbers as u32,
        0,
    ));
}

fn message_chains(
    ctx: &SmellContext<'_>,
    m: &FunctionMetrics,
    cfg: &Smells,
    out: &mut Vec<SmellFinding>,
) {
    if m.max_chain_depth > cfg.max_chain_depth {
        out.push(make(
            SmellKind::MessageChain,
            format!(
                "attribute chain depth {} (threshold {}) — Law of Demeter",
                m.max_chain_depth, cfg.max_chain_depth
            ),
            ctx.path,
            m.start_line,
            &m.name,
            Severity::Warning,
            m.max_chain_depth as u32,
            cfg.max_chain_depth as u32,
        ));
    }
}

fn unnecessary_nested_if(
    ctx: &SmellContext<'_>,
    m: &FunctionMetrics,
    out: &mut Vec<SmellFinding>,
) {
    if m.collapsible_nested_ifs > 0 {
        out.push(make(
            SmellKind::UnnecessaryNestedIf,
            format!(
                "{} nested if(s) can be combined with a boolean AND",
                m.collapsible_nested_ifs
            ),
            ctx.path,
            m.start_line,
            &m.name,
            Severity::Info,
            m.collapsible_nested_ifs as u32,
            0,
        ));
    }
}

/// Advisory: a local assigned `split_variable_min_assigns`+ times either holds
/// distinct concepts or is branch-bound state — both want a single binding
/// (extract a helper that returns the value).
fn split_variables(
    ctx: &SmellContext<'_>,
    m: &FunctionMetrics,
    cfg: &Smells,
    out: &mut Vec<SmellFinding>,
) {
    let min_assigns = cfg.split_variable_min_assigns.max(2);
    for (name, &count) in &m.local_reassigns {
        if count >= min_assigns {
            out.push(make(
                SmellKind::SplitVariable,
                format!(
                    "local `{name}` assigned {count} times — bind it once (extract a helper returning the value)"
                ),
                ctx.path,
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
