use super::*;
use serde_json::json;

#[test]
fn unsubstituted_path_is_an_error_naming_the_template() {
    let err = repository_path(&json!({"path": "${cwd}"})).unwrap_err();
    assert!(err.contains("${cwd}"), "{err}");
}

#[test]
fn missing_directory_is_an_error() {
    let tmp = tempfile::tempdir().unwrap();
    let gone = tmp.path().join("gone");
    let err = repository_path(&json!({"path": gone})).unwrap_err();
    assert!(err.contains("not an existing directory"), "{err}");
}

#[test]
fn unsubstituted_optional_fields_count_as_absent() {
    let args = json!({"session_id": "${session_id}", "transcript_path": ""});
    assert_eq!(templated(&args, "session_id"), None);
    assert_eq!(templated(&args, "transcript_path"), None);
}

#[test]
fn stop_hook_active_accepts_bool_and_substituted_string() {
    assert!(stop_hook_active(&json!({"stop_hook_active": true})));
    assert!(stop_hook_active(&json!({"stop_hook_active": "true"})));
    assert!(!stop_hook_active(
        &json!({"stop_hook_active": "${stop_hook_active}"})
    ));
    assert!(!stop_hook_active(&json!({})));
}
