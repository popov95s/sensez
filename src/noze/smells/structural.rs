//! Structural smells: deep nesting, long parameter lists, too many returns,
//! magic numbers, message chains (Law of Demeter), and split variables.

use super::make;
use crate::config::smells::Smells;
use crate::noze::{Severity, SmellFinding, SmellKind};
use crate::spine::parser::{FunctionUnit, ParsedFile};

pub fn detect(file: &ParsedFile, cfg: &Smells, out: &mut Vec<SmellFinding>) {
    for func in &file.walked.units.functions {
        deep_nesting(func, cfg, file, out);
        long_params(func, cfg, file, out);
        too_many_returns(func, cfg, file, out);
        if cfg.magic_numbers && func.magic_numbers > 0 {
            magic_numbers(func, file, out);
        }
        message_chains(func, cfg, file, out);
        unnecessary_nested_if(func, file, out);
        if cfg.split_variable {
            split_variables(func, cfg, file, out);
        }
    }
}

fn deep_nesting(func: &FunctionUnit, cfg: &Smells, file: &ParsedFile, out: &mut Vec<SmellFinding>) {
    if func.max_nesting > cfg.max_nesting {
        out.push(make(
            SmellKind::DeepNesting,
            format!(
                "nesting depth {} (threshold {})",
                func.max_nesting, cfg.max_nesting
            ),
            &file.path,
            func.start_line,
            &func.name,
            Severity::Warning,
            func.max_nesting as u32,
            cfg.max_nesting as u32,
        ));
    }
}

fn long_params(func: &FunctionUnit, cfg: &Smells, file: &ParsedFile, out: &mut Vec<SmellFinding>) {
    let params = effective_params(func);
    if cfg.long_parameter_list && params > cfg.max_params {
        out.push(make(
            SmellKind::LongParameterList,
            format!("{params} parameters (threshold {})", cfg.max_params),
            &file.path,
            func.start_line,
            &func.name,
            Severity::Warning,
            params as u32,
            cfg.max_params as u32,
        ));
    }
}

fn too_many_returns(
    func: &FunctionUnit,
    cfg: &Smells,
    file: &ParsedFile,
    out: &mut Vec<SmellFinding>,
) {
    if cfg.too_many_returns && func.return_count > cfg.max_returns {
        out.push(make(
            SmellKind::TooManyReturns,
            format!(
                "{} return statements (threshold {})",
                func.return_count, cfg.max_returns
            ),
            &file.path,
            func.start_line,
            &func.name,
            Severity::Warning,
            func.return_count as u32,
            cfg.max_returns as u32,
        ));
    }
}

fn magic_numbers(func: &FunctionUnit, file: &ParsedFile, out: &mut Vec<SmellFinding>) {
    out.push(make(
        SmellKind::MagicNumbers,
        format!("{} magic numeric literal(s)", func.magic_numbers),
        &file.path,
        func.start_line,
        &func.name,
        Severity::Info,
        func.magic_numbers as u32,
        0,
    ));
}

fn message_chains(
    func: &FunctionUnit,
    cfg: &Smells,
    file: &ParsedFile,
    out: &mut Vec<SmellFinding>,
) {
    if func.max_chain_depth > cfg.max_chain_depth {
        out.push(make(
            SmellKind::MessageChain,
            format!(
                "attribute chain depth {} (threshold {}) — Law of Demeter",
                func.max_chain_depth, cfg.max_chain_depth
            ),
            &file.path,
            func.start_line,
            &func.name,
            Severity::Warning,
            func.max_chain_depth as u32,
            cfg.max_chain_depth as u32,
        ));
    }
}

fn unnecessary_nested_if(func: &FunctionUnit, file: &ParsedFile, out: &mut Vec<SmellFinding>) {
    if func.collapsible_nested_ifs > 0 {
        out.push(make(
            SmellKind::UnnecessaryNestedIf,
            format!(
                "{} nested if(s) can be combined with a boolean AND",
                func.collapsible_nested_ifs
            ),
            &file.path,
            func.start_line,
            &func.name,
            Severity::Info,
            func.collapsible_nested_ifs as u32,
            0,
        ));
    }
}

/// Advisory: a local assigned `split_variable_min_assigns`+ times either holds
/// distinct concepts or is branch-bound state — both want a single binding
/// (extract a helper that returns the value).
fn split_variables(
    func: &FunctionUnit,
    cfg: &Smells,
    file: &ParsedFile,
    out: &mut Vec<SmellFinding>,
) {
    let min_assigns = cfg.split_variable_min_assigns.max(2);
    for (name, &count) in &func.local_reassigns {
        if count >= min_assigns {
            out.push(make(
                SmellKind::SplitVariable,
                format!(
                    "local `{name}` assigned {count} times — bind it once (extract a helper returning the value)"
                ),
                &file.path,
                func.start_line,
                &func.name,
                Severity::Info,
                count as u32,
                (min_assigns - 1) as u32,
            ));
        }
    }
}

/// Parameter count excluding a leading `self`/`cls` receiver.
fn effective_params(func: &FunctionUnit) -> usize {
    let skip = matches!(
        func.param_names.first().map(String::as_str),
        Some("self") | Some("cls")
    ) as usize;
    func.param_names.len().saturating_sub(skip)
}
