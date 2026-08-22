use super::model::ImpactPlan;
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
    for runner in &plan.runners {
        let _ = writeln!(
            out,
            "  {}: {} test file(s)",
            runner.kind.label(),
            runner.tests.len()
        );
    }
    for selection in plan.selected.iter().take(20) {
        let file = selection.file.strip_prefix(root).unwrap_or(&selection.file);
        let _ = writeln!(
            out,
            "    {}  {:?} (distance {})",
            file.display(),
            selection.reason,
            selection.distance
        );
    }
    if plan.selected.len() > 20 {
        let _ = writeln!(out, "    … {} more", plan.selected.len() - 20);
    }
    out.trim_end().to_string()
}
