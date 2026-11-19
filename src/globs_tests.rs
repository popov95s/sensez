use super::*;

#[test]
fn valid_globs_match() {
    let set = build_globset(&["**/tests/**".to_string()]).unwrap();
    assert!(set.is_match("src/tests/x.py"));
    assert!(!set.is_match("src/main.py"));
    let python_tests = build_globset(&["**/test_*.py".to_string()]).unwrap();
    assert!(python_tests.is_match("src/test_api.py"));
    assert!(!python_tests.is_match("src/test_helpers/example.py"));
}

#[test]
fn name_patterns_match_exact_or_prefix() {
    assert!(matches_name("register", "register"));
    assert!(!matches_name("register", "registerAll"));
    assert!(matches_name("on*", "onTestEnd"));
    assert!(!matches_name("on*", "render"));
}

#[test]
fn invalid_globs_fail_validation() {
    let err = validate_patterns("exclude", &["[invalid".to_string()]).unwrap_err();
    assert!(err.to_string().contains("invalid glob in exclude"));
}
