//! Cyclomatic and cognitive complexity per function.

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
    for m in metrics {
        let cyclomatic = m.branch_count + 1;
        if cfg.cyclomatic_complexity && cyclomatic > cfg.max_cyclomatic {
            let sev = if cyclomatic > cfg.max_cyclomatic * 2 {
                Severity::Critical
            } else {
                Severity::Warning
            };
            out.push(make(
                SmellKind::HighComplexity,
                format!(
                    "cyclomatic complexity {cyclomatic} (threshold {})",
                    cfg.max_cyclomatic
                ),
                ctx.path,
                m.start_line,
                &m.name,
                sev,
                cyclomatic as u32,
                cfg.max_cyclomatic as u32,
            ));
        }
        if m.cognitive > cfg.max_cognitive {
            let sev = if m.cognitive > cfg.max_cognitive * 2 {
                Severity::Critical
            } else {
                Severity::Warning
            };
            out.push(make(
                SmellKind::HighCognitiveComplexity,
                format!(
                    "cognitive complexity {} (threshold {})",
                    m.cognitive, cfg.max_cognitive
                ),
                ctx.path,
                m.start_line,
                &m.name,
                sev,
                m.cognitive as u32,
                cfg.max_cognitive as u32,
            ));
        }
    }
}
