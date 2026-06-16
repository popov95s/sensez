//! Cyclomatic and cognitive complexity per function.

use super::make;
use crate::config::smells::Smells;
use crate::noze::{Severity, SmellFinding, SmellKind};
use crate::spine::parser::ParsedFile;

pub fn detect(file: &ParsedFile, cfg: &Smells, out: &mut Vec<SmellFinding>) {
    for func in &file.walked.units.functions {
        let cyclomatic = func.branch_count + 1;
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
                &file.path,
                func.start_line,
                &func.name,
                sev,
                cyclomatic as u32,
                cfg.max_cyclomatic as u32,
            ));
        }
        if func.cognitive > cfg.max_cognitive {
            let sev = if func.cognitive > cfg.max_cognitive * 2 {
                Severity::Critical
            } else {
                Severity::Warning
            };
            out.push(make(
                SmellKind::HighCognitiveComplexity,
                format!(
                    "cognitive complexity {} (threshold {})",
                    func.cognitive, cfg.max_cognitive
                ),
                &file.path,
                func.start_line,
                &func.name,
                sev,
                func.cognitive as u32,
                cfg.max_cognitive as u32,
            ));
        }
    }
}
