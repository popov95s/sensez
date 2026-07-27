use super::detect_local;
use crate::config::smells::Smells;
use crate::report::{SmellFinding, SmellKind};
use crate::spine::parser::parse_file;
use std::fs;

fn local(source: &str) -> Vec<SmellFinding> {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("sample.py");
    fs::write(&path, source).unwrap();
    let file = parse_file(&path, 0).unwrap();
    detect_local(&file, &Smells::default())
}

fn has(findings: &[SmellFinding], kind: SmellKind) -> bool {
    findings.iter().any(|finding| finding.kind == kind)
}

#[test]
fn schema_keys_propagate_across_helpers() {
    let source = r#"
def identity(data):
    return data["id"], data["name"]

def routing(data):
    return data["queue"], data["region"]

def handle(payload):
    identity(payload)
    routing(payload)
"#;
    assert!(has(&local(source), SmellKind::ImplicitSchema));
}

#[test]
fn typed_validator_consumption_is_not_loose_typing() {
    let source = r#"
from typing import Any

def decode(payload: dict[str, Any]) -> User:
    return User.model_validate(payload)
"#;
    let findings = local(source);
    assert!(!has(&findings, SmellKind::LooseTyping), "{findings:?}");
    assert!(!has(&findings, SmellKind::ImplicitSchema), "{findings:?}");
}

#[test]
fn typed_json_boundary_is_not_an_implicit_domain_schema() {
    let source = r#"
def parse_user(payload: dict[str, object]) -> User:
    return User(
        payload["id"],
        payload["name"],
        payload["email"],
        payload["team"],
    )
"#;
    let findings = local(source);
    assert!(!has(&findings, SmellKind::ImplicitSchema), "{findings:?}");
}
