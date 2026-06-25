//! Size smells: long function (line span), large class (method count), and
//! heavy nested functions (inner defs that outgrew "simple wrapper").

use super::{make, SmellContext};
use crate::config::smells::Smells;
use crate::noze::{Severity, SmellFinding, SmellKind};
use crate::spine::ir::{ClassUnit, FunctionMetrics};

pub fn detect(
    ctx: &SmellContext<'_>,
    metrics: &[FunctionMetrics],
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
        // A nested def carrying real logic is hidden inside its parent.
        if cfg.max_nested_function_lines > 0
            && m.is_nested
            && lines > cfg.max_nested_function_lines
        {
            out.push(make(
                SmellKind::HeavyNestedFunction,
                format!(
                    "nested function inside `{p}` spans {lines} lines — not a simple wrapper; it cannot be unit-tested in isolation (only reachable through `{p}`), so extract it to module/class level",
                    p = m.parent
                ),
                ctx.path,
                m.start_line,
                &m.name,
                Severity::Warning,
                lines as u32,
                cfg.max_nested_function_lines as u32,
            ));
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
