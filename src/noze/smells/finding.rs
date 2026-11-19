use super::Measure;
use crate::report::{ActionLevel, Severity, SmellFinding, SmellKind};
use std::path::Path;

/// Construct a finding (keeps the per-smell call sites terse).
pub(in crate::noze) fn make(
    kind: SmellKind,
    message: String,
    file: &Path,
    line: usize,
    symbol: &str,
    severity: Severity,
    measure: Measure,
) -> SmellFinding {
    SmellFinding {
        action: ActionLevel::from_severity(severity),
        kind,
        message,
        file: file.to_path_buf(),
        line,
        end_line: 0,
        lines: Vec::new(),
        symbol: symbol.to_string(),
        severity,
        metric: measure.value(),
        threshold: measure.threshold(),
        reason: String::new(),
    }
}
