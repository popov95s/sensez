//! Size smells: long function, narrating comments, large class, and heavy
//! nested functions.

use super::{make, SmellContext};
use crate::config::smells::Smells;
use crate::report::{Severity, SmellFinding, SmellKind};
use crate::spine::ir::{ClassUnit, FunctionUnit};

pub fn detect(
    ctx: &SmellContext<'_>,
    metrics: &[FunctionUnit],
    cfg: &Smells,
    classes: &[ClassUnit],
    out: &mut Vec<SmellFinding>,
) {
    for m in metrics {
        let lines = m.end_line.saturating_sub(m.start_line) + 1;
        if cfg.long_function && lines > cfg.max_function_lines {
            out.push(make(
                SmellKind::LongFunction,
                format!("{lines} lines (threshold {})", cfg.max_function_lines),
                ctx.path,
                m.start_line,
                &m.name,
                Severity::Warning,
                lines as u32,
                cfg.max_function_lines as u32,
            ));
        }
        if cfg.narrating_code {
            narrating_code(ctx, cfg, out, m, lines);
        }
        // A nested def carrying real logic is hidden inside its parent.
        if cfg.max_nested_function_lines > 0
            && m.is_nested
            && lines > cfg.max_nested_function_lines
            && !m.parent_is_test_callback
            && !(m.captures_parent && cfg.nested_function_skip_capturing)
        {
            out.push(heavy_nested_function(ctx, cfg, m, lines));
        }
    }
    for class in classes {
        let methods = class.methods.len();
        if cfg.large_class && methods > cfg.max_class_methods {
            out.push(make(
                SmellKind::LargeClass,
                format!("{methods} methods (threshold {})", cfg.max_class_methods),
                ctx.path,
                class.start_line,
                &class.name,
                Severity::Warning,
                methods as u32,
                cfg.max_class_methods as u32,
            ));
        }
    }
}

fn heavy_nested_function(
    ctx: &SmellContext<'_>,
    cfg: &Smells,
    m: &FunctionUnit,
    lines: usize,
) -> SmellFinding {
    let p = &m.parent;
    let (message, severity) = if m.captures_parent {
        (
            format!("nested function inside `{p}` spans {lines} lines and reads `{p}`'s locals — extract with explicit parameters so it can be tested on its own"),
            Severity::Info,
        )
    } else {
        (
            format!("nested function inside `{p}` spans {lines} lines — not a simple wrapper; it cannot be unit-tested in isolation (only reachable through `{p}`), so extract it to module/class level"),
            Severity::Warning,
        )
    };
    make(
        SmellKind::HeavyNestedFunction,
        message,
        ctx.path,
        m.start_line,
        &m.name,
        severity,
        lines as u32,
        cfg.max_nested_function_lines as u32,
    )
}

fn narrating_code(
    ctx: &SmellContext<'_>,
    cfg: &Smells,
    out: &mut Vec<SmellFinding>,
    m: &FunctionUnit,
    lines: usize,
) {
    let noisy = m.narration_comment_lines + m.code_comment_lines;
    let only_narration = m.code_comment_lines == 0;
    if noisy < cfg.min_comment_lines
        || (only_narration
            && (lines < cfg.narrating_min_function_lines || m.review_risks.is_test_case))
    {
        return;
    }
    let ratio = (noisy * 100).div_ceil(lines.saturating_sub(m.comment_lines).max(1));
    if ratio <= cfg.max_comment_ratio_percent {
        return;
    }
    let message = if m.code_comment_lines >= m.narration_comment_lines {
        format!(
            "{} line(s) of commented-out code, {ratio}% of code lines (threshold {}%) — delete them; version control keeps the history",
            m.code_comment_lines, cfg.max_comment_ratio_percent
        )
    } else {
        format!(
            "{noisy} narrating comment line(s), {ratio}% of code lines (threshold {}%) — prefer clearer names or extracted helpers; keep comments for why",
            cfg.max_comment_ratio_percent
        )
    };
    out.push(make(
        SmellKind::NarratingCode,
        message,
        ctx.path,
        m.first_noisy_comment_line.unwrap_or(m.start_line),
        &m.name,
        Severity::Info,
        ratio as u32,
        cfg.max_comment_ratio_percent as u32,
    ));
}
