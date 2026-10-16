use super::model::{ImpactPlan, RunMode};
use std::fmt::Write;
use std::path::Path;

pub fn terminal(plan: &ImpactPlan, root: &Path) -> String {
    let mut out = String::new();
    let mode = if plan.full_suite {
        "full safety fallback"
    } else {
        "affected"
    };
    let _ = writeln!(
        out,
        "sensez reflexez: {} / {} tests selected ({mode}) in {}ms",
        plan.selected.len(),
        plan.discovered_tests,
        plan.selection_ms
    );
    for reason in &plan.fallback_reasons {
        let _ = writeln!(out, "  safety: {reason}");
    }
    for warning in &plan.config_warnings {
        let config = warning.file.strip_prefix(root).unwrap_or(&warning.file);
        let _ = writeln!(
            out,
            "  warning: {} `{}`: {}; using {}",
            config.display(),
            warning.key,
            warning.reason,
            warning.fallback
        );
    }
    for path in &plan.unmapped_changes {
        let unmapped = path.strip_prefix(root).unwrap_or(path);
        let _ = writeln!(
            out,
            "  unmapped: {} is not in the import graph",
            unmapped.display()
        );
    }
    for runner in &plan.runners {
        let runner_root = runner.root.strip_prefix(root).unwrap_or(&runner.root);
        let _ = match runner.mode {
            RunMode::Full => writeln!(
                out,
                "  {}: full run in ./{}",
                runner.kind.label(),
                runner_root.display()
            ),
            RunMode::Narrowed => writeln!(
                out,
                "  {}: {} test file(s)",
                runner.kind.label(),
                runner.tests.len()
            ),
        };
    }
    for selection in plan.selected.iter().take(20) {
        let file = selection.file.strip_prefix(root).unwrap_or(&selection.file);
        let _ = writeln!(
            out,
            "    {}  {} (distance {})",
            file.display(),
            selection.reason.label(),
            selection.distance
        );
    }
    if plan.selected.len() > 20 {
        let _ = writeln!(out, "    … {} more", plan.selected.len() - 20);
    }
    out.trim_end().to_string()
}
