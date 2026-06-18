use super::*;
use crate::report::{ActionLevel, CloneClass, CloneOccurrence};

#[test]
fn groups_duplication_without_listing_every_occurrence() {
    let root = Path::new("/repo");
    let config = Config::default();
    let mut report = AnalysisReport::default();
    report.duplication.push(CloneClass {
        action: ActionLevel::Advisory,
        token_length: 80,
        occurrences: vec![
            CloneOccurrence {
                file: PathBuf::from("/repo/a.py"),
                start_row: 1,
                end_row: 4,
            },
            CloneOccurrence {
                file: PathBuf::from("/repo/b.py"),
                start_row: 8,
                end_row: 11,
            },
        ],
        hint: None,
    });
    report.duplication.push(CloneClass {
        action: ActionLevel::Advisory,
        token_length: 70,
        occurrences: vec![CloneOccurrence {
            file: PathBuf::from("/repo/c.py"),
            start_row: 1,
            end_row: 3,
        }],
        hint: None,
    });

    let summary = from_report(root, &report, &config);
    let duplication = summary.by_rule.get("duplication").unwrap();
    assert_eq!(summary.total_count, 2);
    assert_eq!(duplication.count, 2);
    assert_eq!(duplication.current_threshold, json!(50));
    assert_eq!(duplication.sample_file_paths, vec!["a.py", "c.py"]);
}
