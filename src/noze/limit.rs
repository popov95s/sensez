use crate::report::{AnalysisReport, LimitPolicy};

/// Cap every pillar, smells included, to the top `max` ranked findings (0 = unlimited).
pub fn limit(report: &mut AnalysisReport, max: usize) {
    if max == 0 {
        return;
    }
    crate::report::limit(report, LimitPolicy::flat(max));
    report.meta.glossary = crate::noze::glossary::for_report(report);
}
