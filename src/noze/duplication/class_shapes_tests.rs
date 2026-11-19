use super::super::test_support::write_files;
use super::*;

#[test]
fn same_class_name_is_info_only() {
    let tmp = tempfile::tempdir().unwrap();
    let files = write_files(
        tmp.path(),
        &[
            ("a.py", "class Customer:\n    x: int\n"),
            ("b.py", "class Customer:\n    y: str\n"),
        ],
    );
    let refs: Vec<_> = files.iter().collect();
    let findings = detect(&refs, true, 4);
    let class_name = findings
        .iter()
        .find(|finding| finding.token_length == 0)
        .expect("same-name class finding");
    assert_eq!(class_name.action, ActionLevel::Info);
    assert_eq!(class_name.occurrences.len(), 2);
}

#[test]
fn same_class_name_is_opt_in() {
    let tmp = tempfile::tempdir().unwrap();
    let files = write_files(
        tmp.path(),
        &[
            ("a.py", "class Customer:\n    x: int\n"),
            ("b.py", "class Customer:\n    y: str\n"),
        ],
    );
    let refs: Vec<_> = files.iter().collect();
    let findings = detect(&refs, false, 4);
    assert!(
        findings.iter().all(|finding| finding.token_length != 0),
        "same-name class finding should be disabled by default"
    );
}

#[test]
fn typed_property_overlap_uses_configured_threshold() {
    let tmp = tempfile::tempdir().unwrap();
    let files = write_files(
        tmp.path(),
        &[
            (
                "a.py",
                "class User:\n    id: int\n    name: str\n    email: str\n    age: int\n",
            ),
            (
                "b.py",
                "class Account:\n    id: int\n    name: str\n    email: str\n    age: int\n",
            ),
        ],
    );
    let refs: Vec<_> = files.iter().collect();
    assert!(
        detect(&refs, false, 5).is_empty(),
        "four shared fields are below threshold five"
    );
    let findings = detect(&refs, false, 4);
    let overlap = findings
        .iter()
        .find(|finding| finding.token_length == 4)
        .expect("property overlap finding");
    assert_eq!(overlap.action, ActionLevel::Advisory);
    assert!(overlap
        .hint
        .as_deref()
        .is_some_and(|hint| hint.contains("id: int")));
}

#[test]
fn typescript_class_fields_join_property_overlap() {
    let tmp = tempfile::tempdir().unwrap();
    let files = write_files(
        tmp.path(),
        &[
            (
                "a.ts",
                "class User { id: number; name: string; role: string; active: boolean; }",
            ),
            (
                "b.ts",
                "class Admin { id: number; name: string; role: string; active: boolean; }",
            ),
        ],
    );
    let refs: Vec<_> = files.iter().collect();
    assert!(
        detect(&refs, false, 4)
            .iter()
            .any(|finding| finding.token_length == 4),
        "TS class fields with matching names/types should overlap"
    );
}
