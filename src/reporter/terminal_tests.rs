use super::*;

#[test]
fn renders_section_headers() {
    let text = render(&AnalysisReport::default(), false);
    assert!(text.contains("Circular imports"));
    assert!(text.contains("Duplication"));
    assert!(text.contains("Dead code candidates"));
    assert!(text.contains("Boundary violations"));
}

#[test]
fn legend_is_opt_in() {
    let mut report = AnalysisReport::default();
    report.meta.glossary = vec![crate::noze::GlossaryEntry {
        term: "cycles".into(),
        title: "Import Cycle".into(),
        explanation: "modules import each other".into(),
    }];
    assert!(!render(&report, false).contains("What these mean"));
    assert!(render(&report, true).contains("What these mean"));
}
